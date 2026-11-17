use otd_core::{NodeId, Value};

use crate::app::OtdApp;

pub(crate) fn select_source_channel_names(app: &OtdApp, id: NodeId) -> Vec<String> {
    if let Some(src) = app.graph.node(id).inputs.first().copied().flatten() {
        let names = channel_names(app, src);
        if !names.is_empty() {
            return names;
        }
    }
    let reference = {
        let rt = app.engine_rt();
        rt.engines
            .eval_param(&app.graph, id, "chop", &app.time)
            .map(|v| v.as_str())
            .unwrap_or_default()
    };
    let reference = reference.trim();
    if reference.is_empty() {
        return Vec::new();
    }
    let src = if otd_core::operator_reference_is_pattern(reference) {
        app.graph
            .chops_matching_reference(id, reference)
            .into_iter()
            .next()
    } else {
        app.graph.find_from(id, reference)
    };
    src.map(|src| channel_names(app, src)).unwrap_or_default()
}

fn channel_names(app: &OtdApp, src: NodeId) -> Vec<String> {
    app.engine_rt()
        .engines
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
    let anchor_top = ui.cursor().top();
    let anchor_bottom = anchor_top + ui.spacing().interact_size.y;
    let menu = ui.menu_button("▾", |ui| {
        if ui.button("*  (all channels)").clicked() {
            *text = "*".into();
            changed = true;
            ui.close();
        }
        ui.separator();
        let screen = ui.ctx().content_rect();
        let below = (screen.bottom() - anchor_bottom - 12.0).max(0.0);
        let above = (anchor_top - screen.top() - 12.0).max(0.0);
        let max_h = (below.max(above) - 36.0).clamp(48.0, screen.height() - 48.0);
        egui::ScrollArea::vertical()
            .id_salt((field_id, "channel-list"))
            .max_height(max_h)
            .show(ui, |ui| {
                ui.set_min_width(140.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                for name in names {
                    if ui.button(name).clicked() {
                        *text = name.clone();
                        changed = true;
                        ui.close();
                    }
                }
            });
    });
    menu.response
        .on_hover_text("Channels on the Select input");
    changed
}
