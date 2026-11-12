//! Window ▸ Memory Locations: the list of markers, stored selections and view-only locations, with
//! a dialog to make or edit one (name, type, lane, colour, comments, and the view settings it
//! stores). Everything goes through the `markers.*` commands, so each change is one undo step.

use super::{SoundApp, Ui, json, mono};
use soundcraft_model::{MarkerKind, MemoryLocation, TrackId};

/// The view settings a location can store, as `markers.add`/`markers.edit` name them, with labels.
const SETTINGS: [(&str, &str, &str); 6] = [
    ("zoom", "Zoom", "Z"),
    ("pre_post_roll", "Pre/Post-Roll", "P"),
    ("track_visibility", "Track Show/Hide", "V"),
    ("track_heights", "Track Heights", "H"),
    ("group_enables", "Group Enables", "G"),
    ("window_config", "Window Configuration", "W"),
];

/// Colours offered for a location (our own palette), plus "none" for the default.
const PALETTE: [[u8; 3]; 8] =
    [[232, 200, 64], [236, 112, 72], [220, 72, 96], [176, 96, 220], [90, 170, 255], [64, 196, 196], [96, 200, 112], [160, 160, 168]];

/// The New/Edit Memory Location dialog's draft, kept in egui's memory while it is open.
#[derive(Clone)]
struct Draft {
    /// The location being edited; `None` for a new one.
    number: Option<u32>,
    name: String,
    kind: MarkerKind,
    /// `None`: a ruler marker on `ruler`; `Some`: that track's lane.
    track: Option<TrackId>,
    ruler: u8,
    color: Option<[u8; 3]>,
    comments: String,
    /// Which settings to store (for an edit: which are stored now, and so which to keep).
    store: [bool; 6],
    /// The stored settings when the dialog opened (an edit stores only what was newly ticked).
    was: [bool; 6],
    window_config: u32,
}

fn draft_id() -> egui::Id {
    egui::Id::unique("memory_location_draft")
}

fn kind_label(k: MarkerKind) -> &'static str {
    match k {
        MarkerKind::Marker => "Marker",
        MarkerKind::Selection => "Selection",
        MarkerKind::None => "None",
    }
}

fn stored(m: &MemoryLocation) -> [bool; 6] {
    let names = m.view.stored();
    SETTINGS.map(|(name, _, _)| names.contains(&name))
}

/// A one-line text field that commits `command` with `{number, key: text}` when it loses focus.
fn text_cell(app: &mut SoundApp, ui: &mut Ui, n: u32, key: &str, value: &str, width: f32, hint: &str) {
    let id = egui::Id::unique(("mem_cell", key, n));
    let mut edit: String = ui.ctx().memory(|m| m.data.get_temp(id)).unwrap_or_else(|| value.to_string());
    let r = crate::widgets::named_text_field(
        ui.add(egui::TextEdit::singleline(&mut edit).desired_width(width).min_size(egui::vec2(width, 0.0)).hint_text(hint)),
        hint,
    );
    if r.changed() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(id, edit.clone()));
    }
    if r.lost_focus() {
        ui.ctx().memory_mut(|m| m.data.remove::<String>(id));
        if edit != value
            && let Err(error) = app.run("markers.edit", json!({"number": n, key: edit}))
        {
            app.ui.status = error;
        }
    }
}

/// The colour swatch, a menu of the palette.
fn colour_cell(app: &mut SoundApp, ui: &mut Ui, n: u32, color: Option<[u8; 3]>) {
    let [r, g, b] = color.unwrap_or(PALETTE[0]);
    let swatch = egui::RichText::new("■").color(egui::Color32::from_rgb(r, g, b));
    ui.menu_button(swatch, |ui| {
        ui.horizontal(|ui| {
            for c in PALETTE {
                let [r, g, b] = c;
                if ui.button(egui::RichText::new("■").color(egui::Color32::from_rgb(r, g, b))).clicked() {
                    let _ = app.run("markers.edit", json!({"number": n, "color": c}));
                    ui.close();
                }
            }
        });
        if ui.button("Default colour").clicked() {
            let _ = app.run("markers.edit", json!({"number": n, "color": null}));
            ui.close();
        }
    })
    .response
    .on_hover_text("Colour");
}

fn row(app: &mut SoundApp, ui: &mut Ui, m: &MemoryLocation) {
    let s = app.engine.session_arc();
    let n = m.number;
    if ui.button(n.to_string()).on_hover_text("Recall").clicked() {
        recall(app, n);
    }
    if ui.small_button("Edit").on_hover_text("Edit name, type, lane, colour, comments and stored settings").clicked() {
        let was = stored(m);
        let draft = Draft {
            number: Some(n),
            name: m.name.clone(),
            kind: m.kind,
            track: m.track,
            ruler: m.ruler.max(1),
            color: m.color,
            comments: m.comments.clone(),
            store: was,
            was,
            window_config: m.view.window_config.unwrap_or(0),
        };
        ui.ctx().data_mut(|d| d.insert_temp(draft_id(), draft));
    }
    if crate::widgets::close_button(ui, "Delete memory location").clicked() {
        let _ = app.run("markers.delete", json!({"number": n}));
    }
    let view = app.ui.memory_view.clone();
    if view.shows("colour") {
        colour_cell(app, ui, n, m.color);
    }
    text_cell(app, ui, n, "rename", &m.name, 110.0, "Name");
    for (column, _) in COLUMNS.iter().filter(|(c, _)| *c != "colour" && view.shows(c)) {
        cell(app, ui, m, column, &s);
    }
    ui.end_row();
}

/// The optional columns, in the order they show, with their titles.
const COLUMNS: [(&str, &str); 8] = [
    ("colour", "Colour"),
    ("location", "Location"),
    ("end", "End"),
    ("length", "Length"),
    ("type", "Type"),
    ("where", "Where"),
    ("stores", "Stores"),
    ("comments", "Comments"),
];

/// The columns the list can be sorted by.
const SORTABLE: [&str; 7] = ["number", "name", "location", "end", "length", "type", "where"];

/// What a row sorts by in `column`: numbers first (a place, a length), then text.
fn sort_key(s: &soundcraft_model::Session, m: &MemoryLocation, column: &str) -> (i64, String) {
    let place = |x: i64| if m.kind == MarkerKind::None { -1 } else { x };
    match column {
        "number" => (i64::from(m.number), String::new()),
        "name" => (0, m.name.to_lowercase()),
        "end" => (if m.kind == MarkerKind::Selection { m.end } else { -1 }, String::new()),
        "length" => (if m.kind == MarkerKind::Selection { m.end.saturating_sub(m.start) } else { -1 }, String::new()),
        "type" => (
            match m.kind {
                MarkerKind::Marker => 0,
                MarkerKind::Selection => 1,
                MarkerKind::None => 2,
            },
            String::new(),
        ),
        "where" => match m.track {
            None => (i64::from(m.ruler), String::new()),
            Some(t) => (10, s.track(t).map(|t| t.name.to_lowercase()).unwrap_or_default()),
        },
        _ => (place(m.start), String::new()),
    }
}

/// A column title: clicking a sortable one sorts by it, again reverses.
fn header(app: &mut SoundApp, ui: &mut Ui, column: &str, title: &str) {
    if !SORTABLE.contains(&column) {
        ui.label(egui::RichText::new(title).strong());
        return;
    }
    let v = &app.ui.memory_view;
    let on = v.sort == column;
    let text = match (on, v.descending) {
        (true, false) => format!("{title} ▴"),
        (true, true) => format!("{title} ▾"),
        _ => title.to_string(),
    };
    if ui.selectable_label(on, egui::RichText::new(text).strong()).on_hover_text("Sort").clicked() {
        sort_by(&mut app.ui.memory_view, column);
    }
}

/// Sorts by `column`; sorting by the same column again reverses it.
fn sort_by(v: &mut crate::MemoryView, column: &str) {
    v.descending = v.sort == column && !v.descending;
    v.sort = column.to_string();
}

/// One optional cell of a row. Location and a selection's End take a typed time, in the main
/// counter's format.
fn cell(app: &mut SoundApp, ui: &mut Ui, m: &MemoryLocation, column: &str, s: &soundcraft_model::Session) {
    let f = |x: i64| s.format_position(x, s.edit.main_counter);
    let n = m.number;
    match column {
        "location" if m.kind != MarkerKind::None => text_cell(app, ui, n, "at", &f(m.start), 90.0, "Location"),
        "end" if m.kind == MarkerKind::Selection => text_cell(app, ui, n, "end", &f(m.end), 90.0, "End"),
        "length" if m.kind == MarkerKind::Selection => {
            ui.label(egui::RichText::new(f(m.end.saturating_sub(m.start))).font(mono(11.0)));
        }
        "type" => {
            ui.label(kind_label(m.kind));
        }
        "where" => {
            let place = match m.track {
                Some(t) => s.track(t).map_or_else(|| "(gone)".to_string(), |t| t.name.clone()),
                None => format!("Ruler {}", m.ruler.max(1)),
            };
            ui.label(place);
        }
        "stores" => {
            let has = stored(m);
            let letters: String = SETTINGS.iter().zip(has).filter(|(_, on)| *on).map(|((_, _, l), _)| *l).collect();
            let tip: Vec<&str> = SETTINGS.iter().zip(has).filter(|(_, on)| *on).map(|((_, label, _), _)| *label).collect();
            let tip = if tip.is_empty() { "No view settings stored".to_string() } else { tip.join(", ") };
            ui.label(egui::RichText::new(letters).font(mono(11.0))).on_hover_text(tip);
        }
        "comments" => text_cell(app, ui, n, "comments", &m.comments, 140.0, "Comments"),
        _ => {
            ui.label("");
        }
    }
}

pub(crate) fn memory_locations(app: &mut SoundApp, ui: &mut Ui) {
    let find_id = egui::Id::unique("memory_locations_find");
    let mut find: String = ui.ctx().data(|d| d.get_temp(find_id)).unwrap_or_default();
    ui.horizontal(|ui| {
        if ui.button("New…").on_hover_text("New memory location, with its settings").clicked() {
            let s = app.engine.session();
            let n = s.markers.iter().map(|m| m.number).max().unwrap_or(0).saturating_add(1);
            let kind = if s.edit.selection.is_empty() { MarkerKind::Marker } else { MarkerKind::Selection };
            let draft = Draft {
                number: None,
                name: format!("Marker {n}"),
                kind,
                track: None,
                ruler: 1,
                color: None,
                comments: String::new(),
                store: [false; 6],
                was: [false; 6],
                window_config: 0,
            };
            ui.ctx().data_mut(|d| d.insert_temp(draft_id(), draft));
        }
        if ui.button("+ Marker").clicked() {
            let _ = app.run("markers.add", json!({}));
        }
        if ui.button("+ Selection").clicked() {
            let _ = app.run("markers.add", json!({"kind": "selection"}));
        }
        let r = crate::widgets::named_text_field(ui.add(egui::TextEdit::singleline(&mut find).desired_width(120.0).hint_text("Find")), "Find");
        if r.changed() {
            ui.ctx().data_mut(|d| d.insert_temp(find_id, find.clone()));
        }
        ui.menu_button("Columns", |ui| {
            for (column, title) in COLUMNS {
                let mut on = app.ui.memory_view.shows(column);
                if ui.checkbox(&mut on, title).changed() {
                    let columns = &mut app.ui.memory_view.columns;
                    columns.retain(|c| c != column);
                    if on {
                        columns.push(column.to_string());
                    }
                }
            }
        });
        ui.menu_button("Delete", |ui| {
            if ui.button("Delete All Memory Locations").clicked() {
                if let Err(error) = app.run("markers.delete_all", json!({})) {
                    app.ui.status = error;
                }
                ui.close();
            }
        });
    });
    let needle = find.trim().to_lowercase();
    let s = app.engine.session_arc();
    let mut rows: Vec<MemoryLocation> = s
        .markers
        .iter()
        .filter(|m| needle.is_empty() || m.name.to_lowercase().contains(&needle) || m.comments.to_lowercase().contains(&needle))
        .cloned()
        .collect();
    let view = app.ui.memory_view.clone();
    rows.sort_by(|a, b| {
        let order = sort_key(&s, a, &view.sort).cmp(&sort_key(&s, b, &view.sort)).then(a.number.cmp(&b.number));
        if view.descending { order.reverse() } else { order }
    });
    let shown: Vec<(&str, &str)> = COLUMNS.iter().copied().filter(|(c, _)| view.shows(c)).collect();
    egui::ScrollArea::both().show(ui, |ui| {
        egui::Grid::new("mem_grid").striped(true).num_columns(5 + shown.len()).show(ui, |ui| {
            header(app, ui, "number", "#");
            for _ in 0..2 {
                ui.label("");
            }
            if view.shows("colour") {
                ui.label("");
            }
            header(app, ui, "name", "Name");
            for (column, title) in shown.iter().filter(|(c, _)| *c != "colour") {
                header(app, ui, column, title);
            }
            ui.end_row();
            for m in &rows {
                row(app, ui, m);
            }
        });
        if rows.is_empty() {
            ui.weak(if needle.is_empty() { "No memory locations. Press Enter to add a marker." } else { "No matches" });
        }
    });
    dialog(app, ui.ctx());
}

/// The New/Edit Memory Location dialog, while a draft is open.
fn dialog(app: &mut SoundApp, ctx: &egui::Context) {
    let Some(mut d) = ctx.data(|x| x.get_temp::<Draft>(draft_id())) else { return };
    let mut open = true;
    let (mut done, mut cancel) = (false, false);
    let title = if d.number.is_some() { "Edit Memory Location" } else { "New Memory Location" };
    let tracks: Vec<(TrackId, String)> = app.engine.session().tracks.iter().map(|t| (t.id, t.name.clone())).collect();
    let configs = app.ui.configurations.len();
    egui::Window::new(title).open(&mut open).collapsible(false).resizable(false).show(ctx, |ui| {
        egui::Grid::new("mem_dialog").num_columns(2).show(ui, |ui| {
            ui.label("Name");
            crate::widgets::named_text_field(ui.add(egui::TextEdit::singleline(&mut d.name).desired_width(200.0)), "Name");
            ui.end_row();
            ui.label("Type");
            ui.horizontal(|ui| {
                for k in [MarkerKind::Marker, MarkerKind::Selection, MarkerKind::None] {
                    // An existing location keeps its type: its place on the timeline depends on it.
                    ui.add_enabled_ui(d.number.is_none(), |ui| ui.radio_value(&mut d.kind, k, kind_label(k)));
                }
            });
            ui.end_row();
            ui.label("Where");
            let place = d.track.and_then(|t| tracks.iter().find(|(id, _)| *id == t)).map_or_else(|| format!("Ruler {}", d.ruler), |(_, n)| n.clone());
            craft_ui::ComboBox::new("mem_where", "").selected_text(place).show_ui(ui, |ui| {
                for r in 1..=5u8 {
                    if ui.selectable_label(d.track.is_none() && d.ruler == r, format!("Ruler {r}")).clicked() {
                        d.track = None;
                        d.ruler = r;
                    }
                }
                for (id, name) in &tracks {
                    if ui.selectable_label(d.track == Some(*id), name).clicked() {
                        d.track = Some(*id);
                    }
                }
            });
            ui.end_row();
            ui.label("Colour");
            ui.horizontal(|ui| {
                for c in PALETTE {
                    let [r, g, b] = c;
                    let mark = if d.color == Some(c) { "▣" } else { "■" };
                    if ui.button(egui::RichText::new(mark).color(egui::Color32::from_rgb(r, g, b))).clicked() {
                        d.color = Some(c);
                    }
                }
                if ui.selectable_label(d.color.is_none(), "Default").clicked() {
                    d.color = None;
                }
            });
            ui.end_row();
            ui.label("Comments");
            crate::widgets::named_text_field(ui.add(egui::TextEdit::multiline(&mut d.comments).desired_rows(2).desired_width(200.0)), "Comments");
            ui.end_row();
        });
        ui.separator();
        ui.label(egui::RichText::new("Also recall").strong());
        for (i, (_, label, _)) in SETTINGS.iter().enumerate() {
            if let Some(on) = d.store.get_mut(i) {
                ui.checkbox(on, *label);
            }
        }
        if d.store[5] {
            ui.horizontal(|ui| {
                ui.label("Configuration");
                ui.add(egui::DragValue::new(&mut d.window_config).range(1..=u32::try_from(configs.max(1)).unwrap_or(1)));
                if configs == 0 {
                    ui.weak("(none saved yet)");
                }
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() {
                done = true;
            }
            if ui.button("Cancel").clicked() {
                cancel = true;
            }
        });
    });
    if cancel {
        open = false;
    }
    if done {
        match apply(app, &d) {
            Ok(()) => open = false,
            Err(error) => app.ui.status = error,
        }
    }
    if open {
        ctx.data_mut(|x| x.insert_temp(draft_id(), d));
    } else {
        ctx.data_mut(|x| x.remove::<Draft>(draft_id()));
    }
}

/// Makes or edits the location as the draft says, in one command.
fn apply(app: &mut SoundApp, d: &Draft) -> Result<(), String> {
    let pick = |want: bool, was: Option<bool>| -> Vec<&str> {
        SETTINGS
            .iter()
            .enumerate()
            .filter(|(i, _)| d.store.get(*i).copied() == Some(want) && was.is_none_or(|w| d.was.get(*i).copied() == Some(w)))
            .map(|(_, (name, _, _))| *name)
            .collect()
    };
    let mut p = json!({"name": d.name, "comments": d.comments, "color": d.color});
    match d.track {
        Some(t) => p["track"] = json!(t.0),
        None => p["ruler"] = json!(d.ruler),
    }
    if d.store[5] {
        p["window_config"] = json!(d.window_config.max(1));
    }
    match d.number {
        None => {
            p["kind"] = json!(kind_label(d.kind).to_lowercase());
            p["store"] = json!(pick(true, None));
            app.run("markers.add", p)?;
        }
        Some(n) => {
            p["number"] = json!(n);
            p["rename"] = json!(d.name);
            if d.track.is_none() {
                p["track"] = serde_json::Value::Null;
            }
            // Newly ticked settings are captured now; unticked ones are forgotten; the rest stay.
            p["store"] = json!(pick(true, Some(false)));
            p["forget"] = json!(pick(false, Some(true)));
            app.run("markers.edit", p)?;
        }
    }
    Ok(())
}

/// Recalls memory location `n`, then the window configuration it stores (the engine recalls the
/// rest). Problems go to the status line.
pub(crate) fn recall(app: &mut SoundApp, n: u32) {
    match app.run("markers.recall", json!({"number": n})) {
        Ok(r) => {
            let config = r.get("window_config").and_then(serde_json::Value::as_u64).and_then(|c| usize::try_from(c).ok());
            if let Some(c) = config {
                let saved = c.checked_sub(1).and_then(|i| app.ui.configurations.get(i)).map(|(_, v)| v.clone());
                let result =
                    saved.ok_or_else(|| format!("there is no window configuration {c}")).and_then(|v| crate::windows::recall_configuration(app, &v));
                if let Err(error) = result {
                    app.ui.status = format!("Memory location {n}: window configuration not recalled: {error}");
                }
            }
        }
        Err(error) => app.ui.status = error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> SoundApp {
        SoundApp::new(soundcraft_engine::demo::demo_engine(), None, crate::Services::default())
    }

    fn draft(number: Option<u32>) -> Draft {
        Draft {
            number,
            name: "Bridge".into(),
            kind: MarkerKind::Selection,
            track: None,
            ruler: 3,
            color: Some(PALETTE[4]),
            comments: "double it".into(),
            store: [true, false, false, true, false, false],
            was: [false; 6],
            window_config: 0,
        }
    }

    #[test]
    fn rows_sort_by_each_column_with_view_only_locations_first_by_place() {
        let mut app = app();
        let e = &mut app.engine;
        e.execute("markers.delete_all", &json!({})).unwrap();
        e.execute("markers.add", &json!({"name": "beta", "at": 5_000})).unwrap();
        e.execute("markers.add", &json!({"name": "Alpha", "kind": "selection", "start": 9_000, "end": 9_500})).unwrap();
        e.execute("markers.add", &json!({"name": "gamma", "kind": "none"})).unwrap();
        e.execute("markers.add", &json!({"name": "delta", "at": 1_000, "track": "Kick"})).unwrap();
        let s = app.engine.session_arc();
        let order = |column: &str| {
            let mut v: Vec<&MemoryLocation> = s.markers.iter().collect();
            v.sort_by(|a, b| sort_key(&s, a, column).cmp(&sort_key(&s, b, column)).then(a.number.cmp(&b.number)));
            v.iter().map(|m| m.name.as_str()).collect::<Vec<_>>()
        };
        assert_eq!(order("location"), ["gamma", "delta", "beta", "Alpha"]);
        assert_eq!(order("name"), ["Alpha", "beta", "delta", "gamma"], "names sort without case");
        assert_eq!(order("number"), ["beta", "Alpha", "gamma", "delta"]);
        assert_eq!(order("length"), ["beta", "gamma", "delta", "Alpha"], "only a selection has a length");
        assert_eq!(order("type"), ["beta", "delta", "Alpha", "gamma"]);
        assert_eq!(order("where"), ["beta", "Alpha", "gamma", "delta"], "rulers before tracks");
    }

    #[test]
    fn sorting_by_a_column_again_reverses_it() {
        let mut v = crate::MemoryView::default();
        assert_eq!((v.sort.as_str(), v.descending), ("location", false));
        sort_by(&mut v, "name");
        assert_eq!((v.sort.as_str(), v.descending), ("name", false));
        sort_by(&mut v, "name");
        assert!(v.descending);
        sort_by(&mut v, "type");
        assert_eq!((v.sort.as_str(), v.descending), ("type", false), "a new column starts ascending");
    }

    #[test]
    fn a_saved_view_with_unknown_names_still_loads() {
        let v: crate::MemoryView = serde_json::from_value(json!({"columns": ["comments", "sparkles"], "sort": "mood"})).unwrap();
        assert!(v.shows("comments") && !v.shows("location"));
        let s = app().engine.session_arc();
        let m = s.markers.first().unwrap();
        assert_eq!(sort_key(&s, m, "mood"), sort_key(&s, m, "location"), "an unknown sort falls back to location");
        assert_eq!(crate::MemoryView::default().columns.len(), 7, "End is off by default");
    }

    #[test]
    fn the_dialog_makes_a_location_in_one_undo_step() {
        let mut app = app();
        app.engine.session_mut().edit.selection = soundcraft_time::Range::new(1_000, 9_000);
        let undo = app.engine.undo_history().len();
        apply(&mut app, &draft(None)).unwrap();
        assert_eq!(app.engine.undo_history().len(), undo + 1);
        let m = app.engine.session().markers.iter().find(|m| m.name == "Bridge").cloned().unwrap();
        assert_eq!((m.kind, m.start, m.end, m.ruler, m.track), (MarkerKind::Selection, 1_000, 9_000, 3, None));
        assert_eq!((m.comments.as_str(), m.color), ("double it", Some(PALETTE[4])));
        assert_eq!(m.view.stored(), vec!["zoom", "track_heights"]);
    }

    #[test]
    fn editing_keeps_what_stays_ticked_captures_new_ticks_and_forgets_unticked() {
        let mut app = app();
        apply(&mut app, &draft(None)).unwrap();
        let m = app.engine.session().markers.iter().find(|m| m.name == "Bridge").cloned().unwrap();
        let stored_zoom = m.view.zoom;
        // The zoom changes after it was stored; leaving Zoom ticked keeps the stored one.
        app.engine.session_mut().edit.zoom.samples_per_px = 7777.0;
        let kick = app.engine.session().track_by_name("Kick").unwrap().id;
        let mut d = draft(Some(m.number));
        d.was = stored(&m);
        d.store = [true, true, false, false, false, false];
        d.name = "Bridge 2".into();
        d.track = Some(kick);
        apply(&mut app, &d).unwrap();
        let m = app.engine.session().markers.iter().find(|x| x.id == m.id).cloned().unwrap();
        assert_eq!((m.name.as_str(), m.track), ("Bridge 2", Some(kick)));
        assert_eq!(m.view.stored(), vec!["zoom", "pre_post_roll"]);
        assert_eq!(m.view.zoom, stored_zoom);

        // Back to a ruler.
        let mut d = draft(Some(m.number));
        d.was = stored(&m);
        d.store = d.was;
        apply(&mut app, &d).unwrap();
        assert_eq!(app.engine.session().markers.iter().find(|x| x.id == m.id).unwrap().track, None);
    }
}
