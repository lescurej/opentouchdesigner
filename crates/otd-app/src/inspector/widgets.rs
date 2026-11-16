use egui::{Color32, RichText, Ui};
use otd_core::Value;

pub fn td_param_label(ui: &mut Ui, label: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(155.0, 22.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.label(RichText::new(label).color(Color32::from_rgb(175, 178, 188)));
        },
    );
}

pub fn panel_param_row(
    ui: &mut Ui,
    label: &str,
    key: &str,
    value: &mut Value,
    range: Option<(f64, f64)>,
    menu: Option<&[otd_core::SelectOption]>,
    button_value: bool,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        td_param_label(ui, label);
        if button_value {
            let mut on = value.as_f64() >= 0.5;
            if td_toggle(ui, &mut on) {
                *value = Value::Float(if on { 1.0 } else { 0.0 });
                changed = true;
            }
        } else if matches!(value, Value::Float(_) | Value::Int(_)) && range.is_some() {
            changed |= td_slider_field(ui, key, value, range);
        } else {
            changed |= value_widget_basic(ui, key, value, range, menu);
        }
    });
    changed
}

pub fn td_toggle(ui: &mut Ui, value: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let off = ui
            .push_id("td_toggle_off", |ui| ui.selectable_label(!*value, "Off"))
            .inner;
        let on = ui
            .push_id("td_toggle_on", |ui| ui.selectable_label(*value, "On"))
            .inner;
        if off.clicked() && *value {
            *value = false;
            changed = true;
        }
        if on.clicked() && !*value {
            *value = true;
            changed = true;
        }
    });
    changed
}

pub fn td_slider_field(
    ui: &mut Ui,
    key: &str,
    value: &mut Value,
    range: Option<(f64, f64)>,
) -> bool {
    match value {
        Value::Float(v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui
                    .add(egui::DragValue::new(v).speed(0.01).max_decimals(4))
                    .changed();
                if let Some((lo, hi)) = range {
                    changed |= ui
                        .add(egui::Slider::new(v, lo..=hi).show_value(false))
                        .changed();
                }
            });
            changed
        }
        Value::Int(v) => {
            let mut changed = false;
            ui.horizontal(|ui| {
                changed |= ui.add(egui::DragValue::new(v).speed(1.0)).changed();
                if let Some((lo, hi)) = range {
                    changed |= ui
                        .add(egui::Slider::new(v, lo as i64..=hi as i64).show_value(false))
                        .changed();
                }
            });
            changed
        }
        _ => value_widget_basic(ui, key, value, range, None),
    }
}

pub fn value_widget_basic(
    ui: &mut Ui,
    key: &str,
    value: &mut Value,
    range: Option<(f64, f64)>,
    menu: Option<&[otd_core::SelectOption]>,
) -> bool {
    if let Some(items) = menu {
        let mut current = value.as_str();
        let mut changed = false;
        egui::ComboBox::from_id_salt(key)
            .selected_text(&current)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for item in items {
                    let label = if item.option.is_empty() {
                        item.value.as_str()
                    } else {
                        item.option.as_str()
                    };
                    let hit = current == item.value || current == item.option;
                    if ui.selectable_label(hit, label).clicked() {
                        current = item.value.clone();
                        changed = true;
                    }
                }
            });
        if changed {
            *value = match value {
                Value::Select(_) => Value::Select(current),
                _ => Value::Str(current),
            };
        }
        return changed;
    }

    match value {
        Value::Float(v) => match range {
            Some((lo, hi)) => ui.add(egui::Slider::new(v, lo..=hi)).changed(),
            None => ui.add(egui::DragValue::new(v).speed(0.01)).changed(),
        },
        Value::Int(v) => match range {
            Some((lo, hi)) => ui
                .add(egui::Slider::new(v, lo as i64..=hi as i64))
                .changed(),
            None => ui.add(egui::DragValue::new(v).speed(1.0)).changed(),
        },
        Value::Bool(v) => td_toggle(ui, v),
        Value::Str(v) => ui.text_edit_singleline(v).changed(),
        Value::Select(v) => ui.text_edit_singleline(v).changed(),
        Value::Vec2(v) => {
            let mut changed = false;
            for c in v.iter_mut() {
                changed |= ui.add(egui::DragValue::new(c).speed(0.01)).changed();
            }
            changed
        }
        Value::Vec3(v) => {
            let mut changed = false;
            for c in v.iter_mut() {
                changed |= ui.add(egui::DragValue::new(c).speed(0.01)).changed();
            }
            changed
        }
        Value::Vec4(v) => {
            if key.contains("color") {
                let mut rgba = [v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32];
                if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                    *v = [
                        rgba[0] as f64,
                        rgba[1] as f64,
                        rgba[2] as f64,
                        rgba[3] as f64,
                    ];
                    return true;
                }
                false
            } else {
                let mut changed = false;
                for c in v.iter_mut() {
                    changed |= ui.add(egui::DragValue::new(c).speed(0.01)).changed();
                }
                changed
            }
        }
    }
}