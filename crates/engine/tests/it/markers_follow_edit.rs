//! Options ▸ Markers Follow Edit: Shuffle edits move markers with the material. Ruler markers
//! follow edits made on every track; a track's lane markers follow that track's edits.

use serde_json::{Value, json};
use soundcraft_engine::Engine;
use soundcraft_model::{Clip, MarkerKind, Source, SourceId, TrackId};

fn run(e: &mut Engine, id: &str, p: Value) {
    if let Err(err) = e.execute(id, &p) {
        panic!("{id}: {err}");
    }
}

/// Two tracks, each with clips at 0..1000 and 2000..3000, in Shuffle; markers at 500 (inside the
/// first clip), 1500 and 2500 on the ruler, and lane markers at 2500 on each track.
fn session(follow: bool) -> (Engine, [TrackId; 2]) {
    let mut e = Engine::default();
    let ids = e.execute("track.new", &json!({"count": 2})).unwrap()["tracks"].clone();
    let t = [TrackId(ids[0].as_u64().unwrap()), TrackId(ids[1].as_u64().unwrap())];
    let s = e.session_mut();
    s.sources.push(Source {
        id: SourceId(7),
        name: "src".into(),
        path: "src.wav".into(),
        channels: 1,
        frames: 10_000,
        sample_rate: 48_000,
        format: soundcraft_audio_io::FileFormat::Wav,
        time_reference: 0,
        unsaved: false,
    });
    for track in t {
        let (a, b) = (s.new_clip_id(), s.new_clip_id());
        s.track_mut(track).and_then(|x| x.playlist_mut()).unwrap().clips =
            vec![Clip::audio(a, "a", SourceId(7), 0, 0, 1000), Clip::audio(b, "b", SourceId(7), 1000, 2000, 1000)];
    }
    for (name, at) in [("in", 500), ("gap", 1500), ("late", 2500)] {
        s.add_marker(name, MarkerKind::Marker, at, at);
    }
    for (i, track) in t.iter().enumerate() {
        let id = s.add_marker(&format!("lane{i}"), MarkerKind::Marker, 2500, 2500);
        s.markers.iter_mut().find(|m| m.id == id).unwrap().track = Some(*track);
    }
    s.edit.markers_follow_edit = follow;
    run(&mut e, "edit.mode", json!({"mode": "shuffle"}));
    (e, t)
}

fn at(e: &Engine, name: &str) -> Option<i64> {
    e.session().markers.iter().find(|m| m.name == name).map(|m| m.start)
}

#[test]
fn a_shuffle_delete_on_every_track_moves_ruler_markers_and_removes_the_ones_it_cuts() {
    let (mut e, t) = session(true);
    run(&mut e, "edit.clear", json!({"tracks": [t[0].0, t[1].0], "start": 0, "end": 1000}));
    assert_eq!(at(&e, "in"), None, "a marker in the removed time goes, as with Cut Time");
    assert_eq!((at(&e, "gap"), at(&e, "late")), (Some(500), Some(1500)));
    assert_eq!((at(&e, "lane0"), at(&e, "lane1")), (Some(1500), Some(1500)), "lane markers follow their tracks");
}

#[test]
fn an_edit_on_some_tracks_moves_only_their_lane_markers() {
    let (mut e, t) = session(true);
    run(&mut e, "edit.clear", json!({"tracks": [t[0].0], "start": 0, "end": 1000}));
    assert_eq!((at(&e, "in"), at(&e, "gap"), at(&e, "late")), (Some(500), Some(1500), Some(2500)), "ruler markers stay");
    assert_eq!((at(&e, "lane0"), at(&e, "lane1")), (Some(1500), Some(2500)));
}

#[test]
fn with_the_option_off_markers_stay_put() {
    let (mut e, t) = session(false);
    run(&mut e, "edit.clear", json!({"tracks": [t[0].0, t[1].0], "start": 0, "end": 1000}));
    assert_eq!((at(&e, "in"), at(&e, "gap"), at(&e, "late")), (Some(500), Some(1500), Some(2500)));
    assert_eq!((at(&e, "lane0"), at(&e, "lane1")), (Some(2500), Some(2500)));
}

#[test]
fn inserting_time_on_every_track_pushes_later_markers() {
    let (mut e, t) = session(true);
    run(&mut e, "edit.insert_silence", json!({"tracks": [t[0].0, t[1].0], "start": 1000, "end": 1200}));
    assert_eq!((at(&e, "in"), at(&e, "gap"), at(&e, "late")), (Some(500), Some(1700), Some(2700)));
    assert_eq!(at(&e, "lane1"), Some(2700));

    let (mut e, t) = session(true);
    run(&mut e, "edit.duplicate", json!({"tracks": [t[0].0, t[1].0], "start": 0, "end": 1000}));
    assert_eq!((at(&e, "in"), at(&e, "gap"), at(&e, "late")), (Some(500), Some(2500), Some(3500)));

    let (mut e, t) = session(true);
    run(&mut e, "edit.repeat", json!({"tracks": [t[0].0, t[1].0], "start": 0, "end": 1000, "count": 2}));
    assert_eq!((at(&e, "gap"), at(&e, "late")), (Some(3500), Some(4500)));
}

#[test]
fn cut_and_paste_on_every_track_carry_the_ruler_markers() {
    let (mut e, t) = session(true);
    run(&mut e, "edit.cut", json!({"tracks": [t[0].0, t[1].0], "start": 0, "end": 1000}));
    assert_eq!((at(&e, "gap"), at(&e, "late")), (Some(500), Some(1500)));
    run(&mut e, "edit.paste", json!({"tracks": [t[0].0, t[1].0], "at": 0}));
    assert_eq!((at(&e, "gap"), at(&e, "late")), (Some(1500), Some(2500)), "pasting the time back puts them back");
}

#[test]
fn a_lane_marker_stops_at_a_locked_clip_like_the_clips_do() {
    let (mut e, t) = session(true);
    let b = e.session().track(t[0]).unwrap().clips()[1].id;
    run(&mut e, "clip.time_lock", json!({"clips": [b.0]}));
    run(&mut e, "edit.clear", json!({"tracks": [t[0].0], "start": 0, "end": 1000}));
    assert_eq!(e.session().track(t[0]).unwrap().clips()[0].start, 2000, "the locked clip stays");
    assert_eq!(at(&e, "lane0"), Some(2500), "and so does the lane marker on it");
}
