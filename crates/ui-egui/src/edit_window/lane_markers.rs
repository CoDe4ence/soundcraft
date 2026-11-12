//! Track-lane markers (View ▸ Marker Displays ▸ Track Lane): a strip along the top of each track's
//! lane shows the markers on that track, and with Folder Lines a folder shows its tracks' markers
//! too. Click recalls, Alt-click deletes, double-click renames one or adds one where there is none.

use super::layout::{sample_at, x_of};
use super::{Color32, Rect, Sense, Session, Shape, SoundApp, Stroke, Tokens, Track, TrackId, Ui, bold, json, pos2, rgb};
use soundcraft_model::MemoryLocation;

/// Height of the marker strip at the top of a track lane.
pub(super) const STRIP_H: f32 = 14.0;

/// The marker colour when a location has none of its own.
pub(super) const DEFAULT_MARKER: Color32 = Color32::from_rgb(232, 200, 64);

pub(super) fn colour(m: &MemoryLocation) -> Color32 {
    m.color.map_or(DEFAULT_MARKER, rgb)
}

/// Whether `track` lies inside `folder`, at any depth (folders nest at most a few levels).
pub(super) fn inside(s: &Session, track: TrackId, folder: TrackId) -> bool {
    let mut current = s.track(track).and_then(|t| t.folder);
    for _ in 0..16 {
        match current {
            Some(f) if f == folder => return true,
            Some(f) => current = s.track(f).and_then(|t| t.folder),
            None => return false,
        }
    }
    false
}

/// The markers a track's lane shows: its own, and with Folder Lines those of the tracks in it.
pub(super) fn shown_on<'a>(s: &'a Session, track: &'a Track) -> impl Iterator<Item = &'a MemoryLocation> + 'a {
    let folders = s.edit.flag("view.marker.folder_lines") && track.kind == soundcraft_model::TrackKind::Folder;
    s.markers.iter().filter(move |m| match m.track {
        Some(t) => t == track.id || (folders && inside(s, t, track.id)),
        None => false,
    })
}

/// Draws the marker strip on `lane` and handles clicks on it.
pub(super) fn strip(app: &mut SoundApp, ui: &mut Ui, track: &Track, lane: Rect, tl: Rect) {
    let s = app.engine.session_arc();
    if !s.edit.flag("view.marker.track_lane") || lane.height() < STRIP_H * 2.0 {
        return;
    }
    let t = Tokens::current();
    let row = Rect::from_min_max(lane.min, pos2(lane.max.x, lane.min.y + STRIP_H));
    let painter = ui.painter().with_clip_rect(row.intersect(ui.clip_rect()));
    painter.rect_filled(row, 0.0, t.marker_ruler.gamma_multiply(0.85));
    let mut marks: Vec<&MemoryLocation> = shown_on(&s, track).collect();
    marks.sort_by_key(|m| m.start);
    // Track Lane Color: each marker's colour runs to the next one.
    if s.edit.flag("view.marker.track_lane_color") {
        for (i, m) in marks.iter().enumerate() {
            let x0 = x_of(&s, tl, m.start);
            let x1 = marks.get(i + 1).map_or(row.max.x, |n| x_of(&s, tl, n.start));
            if x1 > x0 {
                painter.rect_filled(Rect::from_min_max(pos2(x0, row.min.y), pos2(x1, row.max.y)), 0.0, colour(m).gamma_multiply(0.25));
            }
        }
    }
    for m in &marks {
        let x = x_of(&s, tl, m.start);
        let col = colour(m);
        if m.kind == soundcraft_model::MarkerKind::Selection {
            let x1 = x_of(&s, tl, m.end);
            painter.rect_filled(Rect::from_min_max(pos2(x, row.min.y + 2.0), pos2(x1, row.max.y - 2.0)), 1.0, col.gamma_multiply(0.45));
        }
        painter.add(Shape::convex_polygon(
            vec![
                pos2(x - 4.0, row.min.y + 1.0),
                pos2(x + 4.0, row.min.y + 1.0),
                pos2(x + 4.0, row.max.y - 4.0),
                pos2(x, row.max.y - 1.0),
                pos2(x - 4.0, row.max.y - 4.0),
            ],
            col,
            Stroke::NONE,
        ));
        painter.text(pos2(x + 6.0, row.center().y), egui::Align2::LEFT_CENTER, &m.name, bold(10.0), t.marker_text);
    }
    let resp = ui.interact(row, ui.scope_id().with(("lane_markers", track.id.0)), Sense::click());
    let Some(p) = resp.interact_pointer_pos() else { return };
    let hit = marks.iter().find(|m| (x_of(&s, tl, m.start) - p.x).abs() < 6.0).map(|m| (m.id, m.number, m.name.clone()));
    if resp.double_clicked() {
        match hit {
            Some((id, _, name)) => app.dialogs.open_rename_marker(id, &name),
            None => {
                let at = sample_at(&s, tl, p.x).max(0);
                if let Err(error) = app.run("markers.add", json!({"at": at, "track": track.id.0})) {
                    app.ui.status = error;
                }
            }
        }
    } else if resp.clicked()
        && let Some((_, number, _)) = hit
    {
        if ui.input(|i| i.modifiers.alt) {
            let _ = app.run("markers.delete", json!({"number": number}));
        } else {
            crate::panels::memory::recall(app, number);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soundcraft_model::{ChannelFormat, MarkerKind, TrackKind};

    #[test]
    fn a_lane_shows_its_own_markers_and_a_folder_its_tracks_with_folder_lines() {
        let mut s = Session::default();
        let outer = s.add_track(TrackKind::Folder, ChannelFormat::Stereo, Some("Band"));
        let inner = s.add_track(TrackKind::Folder, ChannelFormat::Stereo, Some("Drums"));
        let kick = s.add_track(TrackKind::Audio, ChannelFormat::Mono, Some("Kick"));
        let bass = s.add_track(TrackKind::Audio, ChannelFormat::Mono, Some("Bass"));
        s.track_mut(inner).unwrap().folder = Some(outer);
        s.track_mut(kick).unwrap().folder = Some(inner);
        for (name, track) in [("hit", Some(kick)), ("slide", Some(bass)), ("verse", None)] {
            let id = s.add_marker(name, MarkerKind::Marker, 0, 0);
            s.markers.iter_mut().find(|m| m.id == id).unwrap().track = track;
        }
        let names = |s: &Session, t: TrackId| shown_on(s, s.track(t).unwrap()).map(|m| m.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&s, kick), vec!["hit"]);
        assert_eq!(names(&s, bass), vec!["slide"]);
        assert!(names(&s, outer).is_empty(), "without Folder Lines a folder shows only its own");
        s.edit.flags.insert("view.marker.folder_lines".into());
        assert_eq!(names(&s, outer), vec!["hit"], "at any depth");
        assert_eq!(names(&s, inner), vec!["hit"]);
        assert!(inside(&s, kick, outer) && !inside(&s, bass, outer) && !inside(&s, outer, kick));
    }
}
