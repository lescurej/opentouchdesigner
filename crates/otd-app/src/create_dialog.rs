use egui::Ui;

use crate::app::OtdApp;

pub fn show(app: &mut OtdApp, ui: &Ui) {
    let Some(dialog) = &mut app.create_dialog else {
        return;
    };
    let mut open = true;
    let mut close_requested = false;
    let mut chosen: Option<String> = None;
    let filter = dialog.filter.to_lowercase();
    let world_pos = dialog.world_pos;
    let connect_from = dialog.connect_from;
    let focus = std::mem::take(&mut dialog.focus);

    egui::Window::new("Add operator")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 80.0))
        .show(ui.ctx(), |ui| {
            let edit = ui.add(
                egui::TextEdit::singleline(&mut app.create_dialog.as_mut().unwrap().filter)
                    .hint_text("type to filter…")
                    .desired_width(320.0),
            );
            if focus {
                edit.request_focus();
            }
            ui.separator();
            let matches: Vec<(&str, &str, &str)> = app
                .registry
                .iter()
                .filter(|d| {
                    filter.is_empty()
                        || d.type_name.to_lowercase().contains(&filter)
                        || d.label.to_lowercase().contains(&filter)
                })
                .map(|d| (d.type_name, d.label, d.summary))
                .collect();

            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for (type_name, label, summary) in &matches {
                        if ui
                            .selectable_label(false, format!("{label}  ({type_name})"))
                            .on_hover_text(*summary)
                            .clicked()
                        {
                            chosen = Some(type_name.to_string());
                        }
                    }
                });

            if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Some((type_name, ..)) = matches.first() {
                    chosen = Some(type_name.to_string());
                }
            }
            if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                close_requested = true;
            }
        });
    let open = open && !close_requested;

    if let Some(type_name) = chosen {
        if let Some(new_id) = app.create_node(&type_name, world_pos) {
            if let Some(src) = connect_from.filter(|s| app.graph.contains(*s)) {
                let _ = app.graph.connect(src, new_id, 0);
            }
        }
        app.create_dialog = None;
    } else if !open {
        app.create_dialog = None;
    }
}
