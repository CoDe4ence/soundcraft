//! The timeline selection on its own: what dragging in a timebase ruler makes, and what playback
//! plays. With Link Timeline and Edit Selection on it is the edit selection, across every shown
//! track; off, the edit selection stays as it is.
use super::{CommandSpec, always, bad, position_param};

use crate::{Engine, Result, cmd};
use serde_json::{Value, json};
use soundcraft_time::Range;

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        noundo "edit.select_timeline",
        "Set Timeline Selection",
        [],
        None,
        "{start, end?} — the timeline selection (a timebase ruler drag); linked, also the edit selection across every shown track",
        always,
        select_timeline
    )]
}

fn select_timeline(e: &mut Engine, p: &Value) -> Result<Value> {
    let id = "edit.select_timeline";
    let start = position_param(e, id, p, "start")?.ok_or_else(|| bad(id, "`start` required"))?.max(0);
    let end = position_param(e, id, p, "end")?.unwrap_or(start).max(0);
    let range = Range::new(start.min(end), start.max(end));
    let shown: Vec<_> = {
        let s = e.session();
        s.tracks.iter().filter(|t| crate::edit::shown(s, t)).map(|t| t.id).collect()
    };
    let s = e.session_mut();
    s.edit.timeline_selection = range;
    if s.edit.link_timeline_edit {
        s.edit.selection = range;
        s.edit.selected_tracks = shown;
        s.edit.selected_clips.clear();
    }
    Ok(json!({"timeline": range, "selection": s.edit.selection}))
}

#[cfg(test)]
mod tests {
    use crate::demo::demo_engine;
    use serde_json::json;
    use soundcraft_time::Range;

    #[test]
    fn a_ruler_selection_is_the_edit_selection_on_every_track_only_while_linked() {
        let mut e = demo_engine();
        e.execute("edit.select_timeline", &json!({"start": 9_000, "end": 1_000})).unwrap();
        let ed = &e.session().edit;
        assert_eq!((ed.timeline_selection, ed.selection), (Range::new(1_000, 9_000), Range::new(1_000, 9_000)), "ends in either order");
        let shown = e.session().tracks.iter().filter(|t| crate::edit::shown(e.session(), t)).count();
        assert_eq!(ed.selected_tracks.len(), shown);

        e.execute("options.link_timeline_edit", &json!({"value": false})).unwrap();
        e.execute("edit.select", &json!({"tracks": ["Kick"], "start": 50, "end": 60})).unwrap();
        e.execute("edit.select_timeline", &json!({"start": 2_000, "end": 4_000})).unwrap();
        let ed = &e.session().edit;
        assert_eq!((ed.timeline_selection, ed.selection, ed.selected_tracks.len()), (Range::new(2_000, 4_000), Range::new(50, 60), 1));
        assert!(e.execute("edit.select_timeline", &json!({})).is_err());
    }
}
