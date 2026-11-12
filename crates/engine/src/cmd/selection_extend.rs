//! Extending the edit selection to the session's start or end (Shift+Home, Shift+End).
use super::{CommandSpec, always};

use crate::{Engine, Result, cmd};
use serde_json::{Value, json};
use soundcraft_time::Range;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(noundo "edit.extend_to_start", "Extend Selection to Start", [], Some("Shift+Home"), "{} — from the session start to the selection's end", always, |e, _| {
            extend(e, true)
        }),
        cmd!(noundo "edit.extend_to_end", "Extend Selection to End", [], Some("Shift+End"), "{} — from the selection's start to the end of the session's material", always, |e, _| {
            extend(e, false)
        }),
    ]
}

fn extend(e: &mut Engine, to_start: bool) -> Result<Value> {
    let end = e.session().content_end();
    let s = e.session_mut();
    let sel = s.edit.selection;
    s.edit.selection = if to_start { Range::new(0, sel.end) } else { Range::new(sel.start, sel.end.max(end)) };
    Ok(json!({"selection": s.edit.selection}))
}

#[cfg(test)]
mod tests {
    use crate::demo::demo_engine;
    use serde_json::json;
    use soundcraft_time::Range;

    #[test]
    fn the_selection_extends_to_the_session_start_or_the_end_of_its_material() {
        let mut e = demo_engine();
        let end = e.session().content_end();
        e.execute("edit.select", &json!({"tracks": ["Kick"], "start": 5_000, "end": 9_000})).unwrap();
        e.execute("edit.extend_to_start", &json!({})).unwrap();
        assert_eq!(e.session().edit.selection, Range::new(0, 9_000));
        e.execute("edit.select", &json!({"start": 5_000})).unwrap();
        e.execute("edit.extend_to_end", &json!({})).unwrap();
        assert_eq!(e.session().edit.selection, Range::new(5_000, end));
        e.execute("edit.select", &json!({"start": end + 100, "end": end + 200})).unwrap();
        e.execute("edit.extend_to_end", &json!({})).unwrap();
        assert_eq!(e.session().edit.selection, Range::new(end + 100, end + 200), "already past the end: unchanged");
        assert_eq!(e.session().edit.timeline_selection, e.session().edit.selection, "linked");
    }
}
