use otd_core::Value;

use crate::app::OtdApp;

pub(crate) fn wired_input_chop_channel_names(app: &OtdApp, id: otd_core::NodeId) -> Vec<String> {
    let Some(src) = app
        .graph
        .node(id)
        .inputs
        .first()
        .and_then(|slot| *slot)
    else {
        return Vec::new();
    };
    app.engine_rt().engines
        .chop_data(src)
        .map(|data| data.channels.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default()
}
pub(crate) fn chop_input_channel_picker(
    ui: &mut egui::Ui,
    field_id: &str,
    value: &mut Value,
    names: &[String],
) -> bool {
    let Value::Str(text) = value else {
        return false;
    };
    let mut changed = ui
        .add(egui::TextEdit::singleline(text).id_salt(field_id))
        .changed();
    if names.is_empty() {
        return changed;
    }
    let menu = ui.menu_button("▾", |ui| {
        if ui.button("*  (all channels)").clicked() {
            *text = "*".into();
            changed = true;
            ui.close();
        }
        ui.separator();
        for name in names {
            if ui.button(name).clicked() {
                *text = name.clone();
                changed = true;
                ui.close();
            }
        }
    });
    menu.response
        .on_hover_text("Pick a channel from the wired input CHOP");
    changed
}
