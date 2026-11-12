//! The view settings a memory location stores and recalls (zoom, pre/post-roll, track visibility,
//! track heights, group enables, window configuration): see [`LocationView`].

use super::bad;
use crate::Result;
use serde_json::{Value, json};
use soundcraft_model::{LocationView, Session, StoredZoom};
use soundcraft_time::MAX_POSITION;

/// The names `store` and `forget` take, in the order they are listed.
pub(super) const SETTINGS: [&str; 6] = ["zoom", "pre_post_roll", "track_visibility", "track_heights", "group_enables", "window_config"];

/// Most window configurations a location can name.
const MAX_WINDOW_CONFIG: u64 = 10_000;

/// The setting names in `p[key]` (an array of strings), checked against [`SETTINGS`].
fn names(cmd: &str, p: &Value, key: &str) -> Result<Vec<&'static str>> {
    let Some(v) = p.get(key) else { return Ok(Vec::new()) };
    let list = v.as_array().ok_or_else(|| bad(cmd, format!("`{key}` must be a list of settings: {}", SETTINGS.join(", "))))?;
    list.iter()
        .map(|x| {
            x.as_str()
                .and_then(|n| SETTINGS.iter().find(|s| **s == n).copied())
                .ok_or_else(|| bad(cmd, format!("`{key}`: unknown setting {x}; use {}", SETTINGS.join(", "))))
        })
        .collect()
}

/// The window configuration number in `p.window_config` (1-based), if given.
fn window_config(cmd: &str, p: &Value) -> Result<Option<u32>> {
    match p.get("window_config") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .filter(|n| (1..=MAX_WINDOW_CONFIG).contains(n))
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| bad(cmd, format!("`window_config` must be a configuration number 1..={MAX_WINDOW_CONFIG}"))),
    }
}

/// Applies `p.store` (capture the named settings from the session now) and `p.forget` (clear
/// them) to `view`. Storing `window_config` needs `p.window_config`, the configuration's number.
pub(super) fn update(cmd: &str, s: &Session, p: &Value, view: &mut LocationView) -> Result<()> {
    let store = names(cmd, p, "store")?;
    let forget = names(cmd, p, "forget")?;
    let config = window_config(cmd, p)?;
    if store.contains(&"window_config") && config.is_none() {
        return Err(bad(cmd, "storing `window_config` needs `window_config`: the configuration's number"));
    }
    let mut next = view.clone();
    for name in forget {
        match name {
            "zoom" => next.zoom = None,
            "pre_post_roll" => next.pre_post_roll = None,
            "track_visibility" => next.hidden_tracks = None,
            "track_heights" => next.track_heights = None,
            "group_enables" => next.group_enables = None,
            _ => next.window_config = None,
        }
    }
    for name in store {
        match name {
            "zoom" => next.zoom = Some(StoredZoom { samples_per_px: s.edit.zoom.samples_per_px, scroll: s.edit.zoom.scroll }),
            "pre_post_roll" => next.pre_post_roll = Some((s.edit.pre_roll, s.edit.post_roll)),
            "track_visibility" => next.hidden_tracks = Some(s.tracks.iter().filter(|t| t.hidden).map(|t| t.id).collect()),
            "track_heights" => next.track_heights = Some(s.tracks.iter().map(|t| (t.id, t.height)).collect()),
            "group_enables" => next.group_enables = Some(s.groups.iter().map(|g| (g.id, g.active)).collect()),
            _ => next.window_config = config,
        }
    }
    // A configuration number on its own (no `store`) sets it too.
    if config.is_some() {
        next.window_config = config;
    }
    *view = next;
    Ok(())
}

/// Recalls `view` into the session: only what it stores, only for tracks and groups that still
/// exist, and with stored numbers (which a session file may hold anything in) kept in range.
/// Returns what the UI recalls itself (the window configuration).
pub(super) fn recall(s: &mut Session, view: &LocationView) -> Value {
    if let Some(z) = view.zoom {
        if z.samples_per_px.is_finite() {
            s.edit.zoom.samples_per_px = z.samples_per_px.clamp(0.25, 1_000_000.0);
        }
        s.edit.zoom.scroll = z.scroll.clamp(0, MAX_POSITION);
    }
    if let Some((pre, post)) = view.pre_post_roll {
        s.edit.pre_roll = pre.clamp(0, MAX_POSITION);
        s.edit.post_roll = post.clamp(0, MAX_POSITION);
    }
    if let Some(hidden) = &view.hidden_tracks {
        for t in &mut s.tracks {
            t.hidden = hidden.contains(&t.id);
        }
    }
    if let Some(heights) = &view.track_heights {
        for t in &mut s.tracks {
            if let Some((_, h)) = heights.iter().find(|(id, _)| *id == t.id) {
                t.height = *h;
            }
        }
    }
    if let Some(groups) = &view.group_enables {
        for g in &mut s.groups {
            if let Some((_, on)) = groups.iter().find(|(id, _)| *id == g.id) {
                g.active = *on;
            }
        }
    }
    json!({"window_config": view.window_config, "recalled": view.stored()})
}

#[cfg(test)]
mod tests {
    use crate::demo::demo_engine;
    use serde_json::json;
    use soundcraft_model::TrackHeight;

    fn track(e: &crate::Engine, name: &str) -> soundcraft_model::Track {
        e.session().track_by_name(name).cloned().unwrap()
    }

    #[test]
    fn a_location_recalls_the_view_settings_it_stores() {
        let mut e = demo_engine();
        let g = e.execute("track.group", &json!({"tracks": ["Kick", "Snare"], "name": "Drums"})).unwrap()["group"].as_u64().unwrap();
        e.execute("track.height", &json!({"tracks": ["Kick"], "height": "large"})).unwrap();
        e.execute("track.hide", &json!({"tracks": ["Bass"], "hidden": true})).unwrap();
        {
            let s = e.session_mut();
            s.edit.zoom.samples_per_px = 300.0;
            s.edit.zoom.scroll = 48_000;
            (s.edit.pre_roll, s.edit.post_roll) = (24_000, 12_000);
        }
        let all = json!(["zoom", "pre_post_roll", "track_visibility", "track_heights", "group_enables", "window_config"]);
        let r = e.execute("markers.add", &json!({"name": "Mix view", "kind": "none", "store": all, "window_config": 2})).unwrap();
        let n = r["number"].clone();

        // Change everything, and add a track the location has never seen.
        e.execute("track.height", &json!({"tracks": ["Kick"], "height": "small"})).unwrap();
        e.execute("track.hide", &json!({"tracks": ["Bass"], "hidden": false})).unwrap();
        e.execute("track.hide", &json!({"tracks": ["Snare"], "hidden": true})).unwrap();
        let later = e.execute("track.new", &json!({"count": 1})).unwrap()["tracks"][0].as_u64().unwrap();
        {
            let s = e.session_mut();
            s.edit.zoom.samples_per_px = 4096.0;
            s.edit.zoom.scroll = 0;
            (s.edit.pre_roll, s.edit.post_roll) = (0, 0);
            s.groups.iter_mut().for_each(|x| x.active = false);
            s.edit.playhead = 777;
        }

        let out = e.execute("markers.recall", &json!({"number": n})).unwrap();
        assert_eq!(out["window_config"], 2, "the UI recalls the window configuration");
        assert!(out.get("at").is_none(), "a `none` location has no place to go to");
        let s = e.session();
        assert_eq!(s.edit.playhead, 777);
        assert_eq!((s.edit.zoom.samples_per_px, s.edit.zoom.scroll, s.edit.pre_roll, s.edit.post_roll), (300.0, 48_000, 24_000, 12_000));
        assert_eq!(track(&e, "Kick").height, TrackHeight::Large);
        assert!(track(&e, "Bass").hidden && !track(&e, "Snare").hidden);
        assert!(!s.track(soundcraft_model::TrackId(later)).unwrap().hidden, "a track made later stays shown");
        assert!(s.groups.iter().find(|x| x.id.0 == g).unwrap().active);
    }

    #[test]
    fn a_location_stores_only_what_it_is_asked_to_and_forgets_on_request() {
        let mut e = demo_engine();
        e.session_mut().edit.zoom.samples_per_px = 300.0;
        let n = e.execute("markers.add", &json!({"at": 1000, "store": ["zoom"]})).unwrap()["number"].clone();
        e.session_mut().edit.zoom.samples_per_px = 900.0;
        e.session_mut().edit.pre_roll = 5;
        let out = e.execute("markers.recall", &json!({"number": n})).unwrap();
        assert_eq!((out["at"].as_i64(), e.session().edit.zoom.samples_per_px, e.session().edit.pre_roll), (Some(1000), 300.0, 5));
        assert_eq!(out["recalled"], json!(["zoom"]));

        e.execute("markers.edit", &json!({"number": n, "forget": ["zoom"]})).unwrap();
        e.session_mut().edit.zoom.samples_per_px = 900.0;
        e.execute("markers.recall", &json!({"number": n})).unwrap();
        assert_eq!(e.session().edit.zoom.samples_per_px, 900.0);
    }

    #[test]
    fn bad_settings_are_refused_and_change_nothing() {
        let mut e = demo_engine();
        let before = e.session().markers.len();
        for p in [
            json!({"store": ["colour"]}),
            json!({"store": "zoom"}),
            json!({"store": ["window_config"]}),
            json!({"window_config": 0}),
            json!({"window_config": "two"}),
        ] {
            assert!(e.execute("markers.add", &p).is_err(), "{p}");
        }
        assert_eq!(e.session().markers.len(), before);
        let n = e.session().markers[0].number;
        assert!(e.execute("markers.edit", &json!({"number": n, "forget": ["everything"], "name": "x"})).is_err());
        assert_ne!(e.session().markers[0].name, "x", "a refused edit changes nothing");
    }

    #[test]
    fn stored_numbers_from_a_session_file_are_kept_in_range() {
        let mut e = demo_engine();
        let n = e.execute("markers.add", &json!({"at": 0, "store": ["zoom", "pre_post_roll"]})).unwrap()["number"].as_u64().unwrap();
        let mut s = e.session().clone();
        let m = s.markers.iter_mut().find(|m| u64::from(m.number) == n).unwrap();
        m.view.zoom = Some(soundcraft_model::StoredZoom { samples_per_px: 1e300, scroll: i64::MAX });
        m.view.pre_post_roll = Some((-5, i64::MAX));
        // Saved and opened again, as a hand-edited file would be.
        let reopened = soundcraft_model::Session::from_json(&s.to_json().unwrap()).unwrap();
        *e.session_mut() = reopened;
        e.execute("markers.recall", &json!({"number": n})).unwrap();
        let ed = &e.session().edit;
        assert_eq!(
            (ed.zoom.samples_per_px, ed.zoom.scroll, ed.pre_roll, ed.post_roll),
            (1_000_000.0, soundcraft_time::MAX_POSITION, 0, soundcraft_time::MAX_POSITION)
        );
    }
}
