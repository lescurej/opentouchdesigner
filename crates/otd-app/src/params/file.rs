use egui::{Color32, RichText};
use otd_core::Value;

use crate::app::OtdApp;

pub(crate) fn file_row(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId, key: &str) {
    let node = app.graph.node(id);
    let param = &node.params[key];
    let label = if param.label.is_empty() {
        key.to_string()
    } else {
        param.label.clone()
    };
    let mut path = param.value.as_str();
    let op_type = node.op_type.clone();

    ui.horizontal(|ui| {
        ui.label(label);
        if ui.button("Browse…").clicked() {
            let picked = crate::media::pick_for(&op_type, key);
            if let Some(p) = picked {
                app.edit(&format!("file:{id:?}"));
                let picked = crate::media::stored_path(app, &p);
                let _ = app.graph.set_param(id, key, Value::Str(picked.clone()));
                path = picked;
            }
        }
    });
    let response = ui.add(
        egui::TextEdit::singleline(&mut path)
            .desired_width(f32::INFINITY)
            .hint_text("a path, or drop a file on this node"),
    );
    if response.changed() {
        app.edit(&format!("file:{id:?}"));
        let _ = app.graph.set_param(id, key, Value::Str(path.clone()));
    }

    // Relative paths resolve against the project, which is what makes a
    // bundle portable — worth saying where somebody is about to type one.
    if let Some(dir) = app.graph.base_dir() {
        ui.label(
            RichText::new(format!("relative to {}", dir.display()))
                .weak()
                .small(),
        );
    }
    // The commonest reason a media node shows nothing is that the path is
    // wrong. Say so here rather than making somebody guess at a black frame.
    if !path.trim().is_empty() {
        let resolved = app.graph.resolve_external(path.trim());
        if !resolved.exists() {
            ui.label(
                RichText::new(format!("no file at {}", resolved.display()))
                    .small()
                    .color(Color32::from_rgb(230, 130, 130)),
            );
        }
    }
}
