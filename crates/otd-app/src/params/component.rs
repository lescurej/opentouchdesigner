use egui::RichText;

use crate::app::OtdApp;

pub(crate) fn component_links(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    if app.graph.node(id).family != otd_core::Family::Comp {
        return;
    }
    let external = app.graph.node(id).external.clone();
    let mut clone_of = app.graph.node(id).clone_of.clone().unwrap_or_default();

    ui.separator();
    if let Some(file) = &external {
        ui.horizontal(|ui| {
            ui.label(RichText::new("From file").weak().small());
            ui.label(RichText::new(file).monospace().small());
        });
        ui.label(
            RichText::new("edits here are local until you save the component again")
                .weak()
                .small(),
        );
    }
    ui.horizontal(|ui| {
        ui.label("Clone of");
        if ui
            .add(
                egui::TextEdit::singleline(&mut clone_of)
                    .hint_text("/master_component")
                    .desired_width(180.0),
            )
            .changed()
        {
            let trimmed = clone_of.trim().to_string();
            app.edit(&format!("clone:{id:?}"));
            app.graph.set_clone(
                id,
                if trimmed.is_empty() {
                    None
                } else {
                    Some(&trimmed)
                },
            );
        }
    });
}

pub(crate) fn custom_param_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    if app.graph.node(id).family != otd_core::Family::Comp {
        return;
    }
    ui.separator();
    let custom: Vec<String> = app
        .graph
        .node(id)
        .custom_params()
        .map(|(k, _)| k.clone())
        .collect();

    egui::CollapsingHeader::new(format!("Component parameters ({})", custom.len()))
        .default_open(!custom.is_empty())
        .show(ui, |ui| {
            ui.label(
                RichText::new("read inside this component as parent.<name>")
                    .weak()
                    .small(),
            );
            let mut remove = None;
            for key in &custom {
                ui.horizontal(|ui| {
                    if ui.small_button("✕").on_hover_text("Remove").clicked() {
                        remove = Some(key.clone());
                    }
                    ui.label(RichText::new(format!("parent.{key}")).monospace().small());
                });
            }
            if let Some(key) = remove {
                app.queue_remove_custom_param(id, key);
            }

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_param_name)
                        .hint_text("name")
                        .desired_width(110.0),
                );
                egui::ComboBox::from_id_salt("newparamtype")
                    .selected_text(&app.new_param_type)
                    .width(80.0)
                    .show_ui(ui, |ui| {
                        for t in ["float", "int", "bool", "str", "rgba"] {
                            if ui.selectable_label(app.new_param_type == t, t).clicked() {
                                app.new_param_type = t.to_string();
                            }
                        }
                    });
                if ui.button("Add").clicked() {
                    let name: String = app
                        .new_param_name
                        .trim()
                        .chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>()
                        .to_lowercase();
                    if !name.is_empty() {
                        let param = match app.new_param_type.as_str() {
                            "int" => otd_core::Param::int(0),
                            "bool" => otd_core::Param::bool(false),
                            "str" => otd_core::Param::str(""),
                            "rgba" => otd_core::Param::rgba([1.0, 1.0, 1.0, 1.0]),
                            _ => otd_core::Param::float(0.0).with_range(0.0, 1.0),
                        };
                        app.edit("add parameter");
                        app.document.history.end_gesture();
                        app.graph.add_custom_param(id, &name, param);
                        app.document.status = format!("added parent.{name}");
                        app.new_param_name.clear();
                    }
                }
            });
        });
}