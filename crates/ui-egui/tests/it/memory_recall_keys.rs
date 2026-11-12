//! Recalling a memory location from the keyboard: Period, its number, Period.

use egui::{Key, Modifiers};
use serde_json::json;
use soundcraft_ui_egui::{Services, SoundApp, shortcuts};

fn press(ctx: &egui::Context, app: &mut SoundApp, key: Key) {
    let events =
        [true, false].map(|pressed| egui::Event::Key { key, physical_key: None, pressed, repeat: false, modifiers: Modifiers::NONE }).to_vec();
    ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| shortcuts::handle(app, ui.ctx())).textures_delta.clear();
}

fn app() -> SoundApp {
    let mut engine = soundcraft_engine::Engine::default();
    for at in [1_000, 2_000, 3_000, 4_000, 5_000, 6_000, 7_000, 8_000, 9_000, 10_000, 11_000, 12_000] {
        engine.execute("markers.add", &json!({"at": at})).unwrap();
    }
    SoundApp::new(engine, None, Services::default())
}

fn at(app: &SoundApp) -> i64 {
    app.engine.session().edit.selection.start
}

#[test]
fn period_number_period_recalls_that_location() {
    let ctx = egui::Context::default();
    let mut app = app();
    for k in [Key::Period, Key::Num1, Key::Num2, Key::Period] {
        press(&ctx, &mut app, k);
    }
    assert_eq!(at(&app), 12_000, "two digits: location 12");
    for k in [Key::Period, Key::Num3, Key::Period] {
        press(&ctx, &mut app, k);
    }
    assert_eq!(at(&app), 3_000);
}

#[test]
fn escape_or_another_key_cancels_the_entry() {
    let ctx = egui::Context::default();
    let mut app = app();
    for k in [Key::Period, Key::Num5, Key::Escape, Key::Period] {
        press(&ctx, &mut app, k);
    }
    assert_eq!(at(&app), 0, "Escape cancelled; the next Period starts a new entry");
    press(&ctx, &mut app, Key::Escape);
    for k in [Key::Period, Key::Num5, Key::A, Key::Num6, Key::Period] {
        press(&ctx, &mut app, k);
    }
    assert_ne!(at(&app), 6_000, "A cancelled the entry, so 6 was not typed into it");
}

#[test]
fn an_unknown_number_says_so_and_changes_nothing() {
    let ctx = egui::Context::default();
    let mut app = app();
    for k in [Key::Period, Key::Num9, Key::Num9, Key::Period] {
        press(&ctx, &mut app, k);
    }
    assert_eq!(at(&app), 0);
    assert!(app.ui.status.contains("unknown memory location"), "{}", app.ui.status);
}

#[test]
fn outside_commands_focus_period_always_starts_an_entry() {
    let ctx = egui::Context::default();
    let mut app = app();
    app.engine.session_mut().edit.keyboard_focus = "ruler".into();
    app.engine.session_mut().edit.selection = soundcraft_time::Range::new(0, 500);
    for k in [Key::Period, Key::Num4, Key::Period] {
        press(&ctx, &mut app, k);
    }
    assert_eq!(at(&app), 4_000);
}

#[test]
fn in_commands_focus_period_still_nudges_what_is_selected() {
    let ctx = egui::Context::default();
    let mut app = app();
    assert_eq!(app.engine.session().edit.keyboard_focus, "commands");
    app.engine.execute("track.new", &json!({"count": 1})).unwrap();
    let t = app.engine.session().tracks[0].id.0;
    app.engine.execute("edit.select", &json!({"tracks": [t], "start": 100, "end": 200})).unwrap();
    press(&ctx, &mut app, Key::Period);
    assert!(app.location_entry().is_none(), "something to nudge: Period nudges");
    assert_ne!(app.engine.session().edit.selection.start, 100, "and the nudge happened");
}
