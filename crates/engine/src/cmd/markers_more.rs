//! Memory Locations window commands beyond add, edit, delete and recall.
use super::{CommandSpec, always, bad, track_param};

use crate::{Engine, Result, cmd};
use serde_json::{Value, json};

pub fn specs() -> Vec<CommandSpec> {
    vec![cmd!(
        "markers.delete_all",
        "Delete All Memory Locations",
        [],
        None,
        "{track?: id|name (only that track's lane markers), ruler?: 1..5 (only that ruler's markers)} → {deleted}",
        always,
        delete_all
    )]
}

fn delete_all(e: &mut Engine, p: &Value) -> Result<Value> {
    let id = "markers.delete_all";
    let track = track_param(e, id, p, "track")?;
    let ruler = match p.get("ruler") {
        None => None,
        Some(v) => Some(v.as_u64().filter(|r| (1..=5).contains(r)).ok_or_else(|| bad(id, "`ruler` must be 1..5"))?),
    };
    let s = e.session_mut();
    let before = s.markers.len();
    s.markers.retain(|m| {
        let picked = match (track, ruler) {
            (Some(t), _) => m.track == Some(t),
            (None, Some(r)) => m.track.is_none() && u64::from(m.ruler) == r,
            (None, None) => true,
        };
        !picked
    });
    Ok(json!({"deleted": before - s.markers.len()}))
}

#[cfg(test)]
mod tests {
    use crate::demo::demo_engine;
    use serde_json::json;

    #[test]
    fn delete_all_takes_everything_or_one_lane_and_undoes() {
        let mut e = demo_engine();
        e.execute("markers.add", &json!({"at": 10, "ruler": 2})).unwrap();
        e.execute("markers.add", &json!({"at": 20, "track": "Kick"})).unwrap();
        let all = e.session().markers.len();
        assert_eq!(e.execute("markers.delete_all", &json!({"ruler": 2})).unwrap()["deleted"], 1);
        assert_eq!(e.execute("markers.delete_all", &json!({"track": "Kick"})).unwrap()["deleted"], 1);
        assert_eq!(e.session().markers.len(), all - 2);
        assert!(e.execute("markers.delete_all", &json!({"ruler": 7})).is_err());
        assert_eq!(e.execute("markers.delete_all", &json!({})).unwrap()["deleted"].as_u64(), Some(all as u64 - 2));
        assert!(e.session().markers.is_empty());
        e.undo();
        assert_eq!(e.session().markers.len(), all - 2);
    }
}
