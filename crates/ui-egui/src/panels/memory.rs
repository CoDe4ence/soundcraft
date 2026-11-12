use super::{SoundApp, Ui, json, mono};

/// One recallable memory-location row: number button, editable name, time, length, kind and delete.
fn memory_location_row(app: &mut SoundApp, ui: &mut Ui, n: u32, name: &str, loc: &str, len: &str, kind: &str) {
    if ui.button(n.to_string()).on_hover_text("Recall").clicked() {
        recall(app, n);
    }
    let key = egui::Id::unique(("mem_name", n));
    let mut edit: String = ui.ctx().memory(|m| m.data.get_temp(key)).unwrap_or_else(|| name.to_string());
    let r = crate::widgets::named_text_field(ui.add(egui::TextEdit::singleline(&mut edit).desired_width(140.0)), "Name");
    if r.changed() {
        ui.ctx().memory_mut(|m| m.data.insert_temp(key, edit.clone()));
    }
    if r.lost_focus() && edit != name {
        let _ = app.run("markers.edit", json!({"number": n, "rename": edit}));
        ui.ctx().memory_mut(|m| m.data.remove::<String>(key));
    }
    ui.label(egui::RichText::new(loc).font(mono(11.0)));
    ui.label(egui::RichText::new(len).font(mono(11.0)));
    ui.label(kind);
    if crate::widgets::close_button(ui, "Delete marker").clicked() {
        let _ = app.run("markers.delete", json!({"number": n}));
    }
    ui.end_row();
}

pub(crate) fn memory_locations(app: &mut SoundApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if ui.button("+ Marker").clicked() {
            let _ = app.run("markers.add", json!({}));
        }
        if ui.button("+ Selection").clicked() {
            let _ = app.run("markers.add", json!({"kind": "selection"}));
        }
    });
    let rows: Vec<(u32, String, String, String, String)> = {
        let s = app.engine.session();
        let f = |x: i64| s.format_position(x, s.edit.main_counter);
        s.markers
            .iter()
            .map(|m| {
                let kind = match m.kind {
                    soundcraft_model::MarkerKind::Marker => "marker",
                    soundcraft_model::MarkerKind::Selection => "selection",
                    soundcraft_model::MarkerKind::None => "none",
                };
                let len = if m.kind == soundcraft_model::MarkerKind::Selection { f(m.end - m.start) } else { String::new() };
                (m.number, m.name.clone(), f(m.start), len, kind.to_string())
            })
            .collect()
    };
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("mem_grid").striped(true).num_columns(6).show(ui, |ui| {
            for h in ["#", "Name", "Location", "Length", "Type", ""] {
                ui.label(egui::RichText::new(h).strong());
            }
            ui.end_row();
            for (n, name, loc, len, kind) in &rows {
                memory_location_row(app, ui, *n, name, loc, len, kind);
            }
        });
    });
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
