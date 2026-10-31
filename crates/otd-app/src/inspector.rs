use egui::{Color32, RichText, Ui};
use otd_core::indexmap::IndexMap;
use otd_core::{Family, NodeId, Param, Value};

use crate::app::OtdApp;

const TAB_ORDER: &[&str] = &["Play", "Image", "Trim", "Tune", "Common", "Parameters"];

const PANEL_TAB_ORDER: &[&str] = &[
    "Button",
    "Slider",
    "Field",
    "Text",
    "Layout",
    "Panel",
    "Look",
    "Children",
    "Drag/Drop",
    "Extensions",
    "Common",
];

pub fn sort_pages(pages: &mut Vec<String>, panel: bool) {
    let order = if panel {
        PANEL_TAB_ORDER
    } else {
        TAB_ORDER
    };
    pages.sort_by(|a, b| {
        let ia = order.iter().position(|&p| p == a.as_str()).unwrap_or(999);
        let ib = order.iter().position(|&p| p == b.as_str()).unwrap_or(999);
        ia.cmp(&ib).then_with(|| a.cmp(b))
    });
    pages.dedup();
}

pub fn collect_pages(params: &IndexMap<String, Param>, panel: bool) -> Vec<String> {
    let mut pages: Vec<String> = params
        .values()
        .filter_map(|p| {
            let page = p.page();
            (!page.is_empty()).then(|| page.to_string())
        })
        .collect();
    if panel {
        for extra in ["Extensions", "Common"] {
            if !pages.iter().any(|p| p == extra) {
                pages.push(extra.into());
            }
        }
    }
    if pages.is_empty() {
        return vec!["Parameters".into()];
    }
    sort_pages(&mut pages, panel);
    pages
}

pub fn ordered_param_keys(op_type: &str, params: &IndexMap<String, Param>, page: &str) -> Vec<String> {
    let table: &[&str] = match op_type {
        otd_engine::panel::BUTTON if page == "Button" => {
            &["label", "value", "mode"]
        }
        otd_engine::panel::SLIDER if page == "Slider" => {
            &["value", "min", "max", "orientation"]
        }
        otd_engine::panel::FIELD if page == "Field" => &["text"],
        otd_engine::panel::TEXT if page == "Text" => &["text", "fontsize"],
        _ if page == "Layout" => &[
            "x", "y", "w", "h", "hmode", "leftanchor", "leftoffset", "rightanchor", "rightoffset",
            "horigin", "hfillweight", "vmode", "bottomanchor", "bottomoffset", "topanchor",
            "topoffset", "vorigin", "vfillweight", "fixedaspect", "aspect", "layer",
            "postoffsetx", "postoffsety", "alignallow", "alignorder",
        ],
        _ if page == "Panel" => &["display", "enable", "clickthrough"],
        _ if page == "Look" => {
            &[
                "opacity", "bgcolorr", "bgcolorg", "bgcolorb", "bgalpha", "topfill", "composite",
                "borderover",
            ]
        }
        _ if page == "Children" => {
            &[
                "align", "spacing", "alignmax", "marginl", "marginr", "margint", "marginb",
                "justifymethod", "justifyh", "justifyv", "fit", "scalex", "scaley", "offsetx",
                "offsety", "crop",
            ]
        }
        _ => &[],
    };
    let mut keys: Vec<String> = table
        .iter()
        .filter(|k| params.contains_key(**k))
        .map(|s| (*s).to_string())
        .collect();
    if keys.is_empty() {
        keys = params
            .iter()
            .filter(|(_, p)| p.page() == page || (page == "Parameters" && p.page().is_empty()))
            .map(|(k, _)| k.clone())
            .collect();
        keys.sort();
    }
    keys
}

pub fn inspector_header_title(app: &OtdApp, id: NodeId) -> String {
    op_display_label(app, id)
}

pub fn header_bar(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    let family = app.graph.node(id).family;
    let c = family.color();
    let base = Color32::from_rgb(c[0], c[1], c[2]);
    let header_bg = base.gamma_multiply(0.38);
    let name_bg = base.gamma_multiply(0.52);
    let title = inspector_header_title(app, id);
    let mut name = app.graph.node(id).name.clone();

    egui::Frame::new()
        .fill(header_bg)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .strong()
                        .color(Color32::from_rgb(240, 240, 245)),
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
                            app.try_set_node_name(id, &name);
                        }
                    });
            });
        });

    ui.add_space(2.0);
}

pub fn panel_common_tab(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    ui.spacing_mut().item_spacing.y = 4.0;
    let mut flags: Vec<(&str, bool, crate::canvas::input::SelectionFlag)> = vec![
        (
            "Bypass",
            app.graph.node(id).flags.bypass,
            crate::canvas::input::SelectionFlag::Bypass,
        ),
        (
            "Viewer",
            app.graph.node(id).flags.viewer,
            crate::canvas::input::SelectionFlag::Viewer,
        ),
        (
            "Clone Immune",
            app.graph.node(id).flags.clone_immune,
            crate::canvas::input::SelectionFlag::CloneImmune,
        ),
        (
            "Viewer Active",
            app.graph.node(id).flags.viewer_active,
            crate::canvas::input::SelectionFlag::ViewerActive,
        ),
        (
            "Lock",
            app.graph.node(id).flags.lock,
            crate::canvas::input::SelectionFlag::Lock,
        ),
    ];
    if app.graph.node(id).family != Family::Comp && app.graph.node(id).family != Family::Mat {
        flags.insert(
            3,
            (
                "Display",
                app.graph.node(id).flags.bg_display,
                crate::canvas::input::SelectionFlag::Display,
            ),
        );
    }
    for (label, on, flag) in flags {
        let mut v = on;
        ui.horizontal(|ui| {
            td_param_label(ui, label);
            if td_toggle(ui, &mut v) {
                if v != on {
                    crate::canvas::input::toggle_flag_at(app, flag, id);
                }
            }
        });
    }
    if app.graph.node(id).family == Family::Comp {
        let mut perform = app.perform_ui() == Some(id);
        ui.horizontal(|ui| {
            td_param_label(ui, "Perform UI");
            if td_toggle(ui, &mut perform) && perform != (app.perform_ui() == Some(id)) {
                app.toggle_perform_ui(id);
            }
        });
    }
}

pub fn panel_extensions_tab(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    crate::params::component_links(app, ui, id);
    crate::params::custom_param_editor(app, ui, id);
}

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
    menu: Option<&[String]>,
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

pub fn page_tabs(app: &mut OtdApp, ui: &mut Ui, id: NodeId, pages: &[String]) -> String {
    let active = app
        .param_inspector_page
        .get(&id)
        .cloned()
        .filter(|p| pages.contains(p))
        .unwrap_or_else(|| pages.first().cloned().unwrap_or_else(|| "Parameters".into()));

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        for page in pages {
            let selected = *page == active;
            let label = RichText::new(page).size(13.0).color(if selected {
                Color32::from_rgb(235, 236, 240)
            } else {
                Color32::from_rgb(120, 122, 132)
            });
            let resp = ui.selectable_label(selected, label);
            if resp.clicked() {
                app.param_inspector_page.insert(id, page.clone());
            }
            if selected {
                let r = resp.rect;
                ui.painter().hline(
                    r.x_range(),
                    r.bottom() + 2.0,
                    egui::Stroke::new(2.0, Color32::from_rgb(220, 222, 230)),
                );
            }
        }
    });
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);
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
    if otd_dat::callbacks::uses_external_callbacks_dat(&app.graph, id) {
        return false;
    }
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
