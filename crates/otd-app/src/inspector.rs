use egui::{Color32, RichText, Ui};
use otd_core::indexmap::IndexMap;
use otd_core::{Family, NodeId, Param, Value};

use crate::app::OtdApp;

const TAB_ORDER: &[&str] = &["Play", "Image", "Trim", "Tune", "Common", "Parameters"];

pub fn sort_pages(pages: &mut Vec<String>) {
    pages.sort_by(|a, b| {
        let ia = TAB_ORDER.iter().position(|&p| p == a.as_str()).unwrap_or(999);
        let ib = TAB_ORDER.iter().position(|&p| p == b.as_str()).unwrap_or(999);
        ia.cmp(&ib).then_with(|| a.cmp(b))
    });
    pages.dedup();
}

pub fn collect_pages(params: &IndexMap<String, Param>) -> Vec<String> {
    let mut pages: Vec<String> = params
        .values()
        .filter_map(|p| {
            let page = p.page();
            (!page.is_empty()).then(|| page.to_string())
        })
        .collect();
    if pages.is_empty() {
        return vec!["Parameters".into()];
    }
    sort_pages(&mut pages);
    pages
}

pub fn header_bar(app: &mut OtdApp, ui: &mut Ui, id: NodeId, op_label: &str) {
    let header_bg = Color32::from_rgb(58, 48, 72);
    let name_bg = Color32::from_rgb(72, 58, 88);
    let mut name = app.graph.node(id).name.clone();
    let parent = app
        .graph
        .node(id)
        .parent
        .unwrap_or_else(|| app.graph.root());

    egui::Frame::new()
        .fill(header_bg)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(op_label)
                        .strong()
                        .color(Color32::from_rgb(220, 215, 235)),
                );
                ui.add_space(6.0);
                egui::Frame::new()
                    .fill(name_bg)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut name)
                                    .desired_width(ui.available_width().max(80.0))
                                    .frame(egui::Frame::NONE),
                            )
                            .changed()
                        {
                            let trimmed = name.trim().to_string();
                            if !trimmed.is_empty() && !app.graph.name_taken(parent, &trimmed) {
                                app.edit(&format!("rename:{id:?}"));
                                app.graph.node_mut_quiet(id).name = trimmed;
                            }
                        }
                    });
            });
        });

    ui.add_space(4.0);
    flag_toolbar(app, ui, id);
    ui.add_space(6.0);
}

fn flag_toolbar(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    ui.horizontal(|ui| {
        let bypass = app.graph.node(id).flags.bypass;
        if ui
            .selectable_label(bypass, "Bypass")
            .on_hover_text("Pass input through (B)")
            .clicked()
        {
            app.edit("bypass");
            app.history.end_gesture();
            app.graph.node_mut(id).flags.bypass = !bypass;
        }

        let display = app.graph.node(id).flags.display;
        if ui
            .selectable_label(display, "Display")
            .on_hover_text("Show live preview on the node body")
            .clicked()
        {
            app.edit("display flag");
            app.history.end_gesture();
            app.graph.node_mut(id).flags.display = !display;
        }

        let lock = app.graph.node(id).flags.lock;
        if ui
            .selectable_label(lock, "Lock")
            .on_hover_text("Block edits and wiring")
            .clicked()
        {
            app.edit("lock");
            app.history.end_gesture();
            app.graph.node_mut(id).flags.lock = !lock;
        }

        let viewer_active = app.viewer == Some(id);
        if ui
            .selectable_label(viewer_active, "Viewer Active")
            .on_hover_text("Drive the main output viewer")
            .clicked()
        {
            app.viewer = if viewer_active { None } else { Some(id) };
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Set output").clicked() {
                app.viewer = Some(id);
            }
        });
    });
}

pub fn page_tabs(app: &mut OtdApp, ui: &mut Ui, id: NodeId, pages: &[String]) -> String {
    let active = app
        .param_inspector_page
        .get(&id)
        .cloned()
        .filter(|p| pages.contains(p))
        .unwrap_or_else(|| pages.first().cloned().unwrap_or_else(|| "Parameters".into()));

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        for page in pages {
            let selected = *page == active;
            let label = RichText::new(page).color(if selected {
                Color32::from_rgb(235, 236, 240)
            } else {
                Color32::from_rgb(130, 132, 140)
            });
            if ui.selectable_label(selected, label).clicked() {
                app.param_inspector_page.insert(id, page.clone());
            }
            if selected {
                let r = ui.min_rect();
                ui.painter().hline(
                    r.x_range(),
                    r.bottom(),
                    egui::Stroke::new(1.5, Color32::from_rgb(200, 200, 210)),
                );
            }
        }
    });
    ui.add_space(6.0);
    app.param_inspector_page
        .get(&id)
        .cloned()
        .unwrap_or(active)
}

pub fn param_on_page(param: &Param, page: &str, has_pages: bool) -> bool {
    if !has_pages || page == "Parameters" {
        return true;
    }
    param.page() == page
}

pub fn td_toggle(ui: &mut Ui, value: &mut bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let off = ui.selectable_label(!*value, "Off");
        let on = ui.selectable_label(*value, "On");
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
                    changed |= ui.add(egui::Slider::new(v, lo..=hi).show_value(false)).changed();
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
    menu: Option<&[String]>,
) -> bool {
    if let Some(items) = menu {
        let mut current = value.as_str();
        let mut changed = false;
        egui::ComboBox::from_id_salt(key)
            .selected_text(&current)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for item in items {
                    if ui.selectable_label(current == *item, item).clicked() {
                        current = item.clone();
                        changed = true;
                    }
                }
            });
        if changed {
            *value = Value::Str(current);
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

pub fn op_display_label(app: &OtdApp, id: NodeId) -> String {
    let op_type = &app.graph.node(id).op_type;
    app.registry
        .get(op_type)
        .map(|d| d.label.to_string())
        .unwrap_or_else(|| op_type.clone())
}

pub fn is_python_source(app: &OtdApp, id: NodeId) -> bool {
    app.graph
        .node(id)
        .params
        .get("source")
        .is_some_and(|p| p.is_script())
}

pub fn is_shader_source(app: &OtdApp, id: NodeId) -> bool {
    if app.graph.node(id).family != Family::Top {
        return false;
    }
    let op = &app.graph.node(id).op_type;
    op == "glslTOP"
        || app
            .graph
            .node(id)
            .params
            .get("language")
            .map(|p| p.value.as_str() == "glsl")
            .unwrap_or(op == "glslTOP")
}
