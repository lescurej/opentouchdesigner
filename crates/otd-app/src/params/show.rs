use egui::{Color32, RichText};
use otd_core::{ParamMode, Value};

use crate::app::OtdApp;

use super::{channel_expr_snippet, format_value, migrate_legacy_modes};
use super::channels::{chop_input_channel_picker, select_source_channel_names};
use super::component::{component_links, custom_param_editor};
use super::constant_chop::constant_chop_rows;
use super::curve::curve_editor;
use super::dat::text_editor;
use super::file::file_row;
use super::script_shader::{script_editor, shader_editor};
use super::ChannelDrag;

pub fn show(app: &mut OtdApp, ui: &mut egui::Ui) {
    ui.set_min_width(ui.available_width().max(480.0));
    let Some(id) = app.canvas.selected.filter(|s| app.graph.contains(*s)) else {
        return;
    };

    let op_type = app.graph.node(id).op_type.clone();
    let locked = app.graph.node(id).flags.lock;
    let is_panel = otd_engine::panel::is_panel_type(&op_type);

    crate::inspector::header_bar(app, ui, id);

    if locked {
        ui.label(
            RichText::new("Locked")
                .small()
                .color(Color32::from_rgb(220, 180, 100)),
        );
    } else if let Some(msg) = app.engine_rt().engines.node_message(&app.graph, id) {
        ui.label(
            RichText::new(&msg.text)
                .small()
                .color(crate::canvas::node_message::text_color(msg.level)),
        );
    }

    migrate_legacy_modes(app, id);

    if !is_panel {
        let path = app.graph.path(id);
        let summary = app
            .registry
            .get(&op_type)
            .map(|d| d.summary.to_string())
            .unwrap_or_default();
        ui.label(RichText::new(&path).weak().small());
        ui.label(RichText::new(summary).weak().small());
        if let Some(note) = otd_engine::docs::note(&op_type) {
            ui.collapsing("How this works", |ui| {
                for para in note.split("\n\n") {
                    let plain = para.replace("**", "").replace('`', "");
                    ui.label(RichText::new(plain).small());
                    ui.add_space(2.0);
                }
            });
        }
        component_links(app, ui, id);
        custom_param_editor(app, ui, id);
        ui.separator();
    }

    let primary = op_type.rsplit('/').next().unwrap_or("");
    let pages = crate::inspector::collect_pages(&app.graph.node(id).params, is_panel, primary);
    let has_pages = pages.first().is_some_and(|p| p != "Parameters");
    let active_page = crate::inspector::page_tabs(app, ui, id, &pages);

    if is_panel && active_page == "Extensions" {
        ui.add_enabled_ui(!locked, |ui| {
            crate::inspector::panel_extensions_tab(app, ui, id);
        });
        return;
    }
    if is_panel && active_page == "Common" {
        ui.add_enabled_ui(!locked, |ui| {
            crate::inspector::panel_common_tab(app, ui, id);
        });
        return;
    }

    let keys = if is_panel {
        crate::inspector::ordered_param_keys(&op_type, &app.graph.node(id).params, &active_page)
    } else {
        app.graph.node(id).params.keys().cloned().collect()
    };
    ui.add_enabled_ui(!locked, |ui| {
    ui.spacing_mut().item_spacing.y = 2.0;
    if op_type == otd_chop::ops::CONSTANT && (active_page == "Constant" || !has_pages) {
        constant_chop_rows(app, ui, id);
    }
    for key in keys {
        if !crate::inspector::param_on_page(
            &app.graph.node(id).params[&key],
            &active_page,
            has_pages,
            primary,
        )
        {
            continue;
        }
        if key == "source" {
            if otd_dat::callbacks::uses_external_callbacks_dat(&app.graph, id) {
                ui.label(
                    RichText::new("Python callbacks are edited in the linked Callbacks DAT.")
                        .weak()
                        .small(),
                );
                continue;
            }
            if crate::inspector::is_python_source(app, id) {
                script_editor(app, ui, id);
            } else if crate::inspector::is_shader_source(app, id) {
                shader_editor(app, ui, id);
            }
            continue;
        }
        // Keyframes get a curve editor rather than a text field. The text is
        // still there underneath and still editable — twenty keys at regular
        // intervals is faster typed than dragged — but a curve is not
        // something you read as numbers.
        if key == "keys" && app.graph.node(id).op_type == "animationCHOP" {
            curve_editor(app, ui, id);
            continue;
        }
        if op_type == otd_chop::ops::CONSTANT
            && (key == "rows" || key.starts_with("name") || key.starts_with("value"))
        {
            continue;
        }
        // A DAT's contents deserve the same room as a shader's source.
        if key == "text" && app.graph.node(id).family == otd_core::Family::Dat {
            text_editor(app, ui, id);
            continue;
        }
        // A file parameter gets a Browse button. Typing an absolute path
        // from memory is nobody's idea of patching.
        if app.graph.node(id).params[&key].is_file_ref()
            || (key == "file" && app.graph.node(id).op_type == otd_gpu::ops::MOVIE_OUT)
        {
            file_row(app, ui, id, &key);
            continue;
        }

        let (label, mode, mut value, mut expression, range, menu, error, evaluated) = {
            let p = &app.graph.node(id).params[&key];
            let label = if p.label.is_empty() {
                key.clone()
            } else {
                p.label.clone()
            };
            let ui_mode = match p.mode {
                ParamMode::Constant | ParamMode::Expression => p.mode,
                _ => ParamMode::Expression,
            };
            let (evaluated, eval_err) = if ui_mode == ParamMode::Expression {
                let rt = app.engine_rt();
                match rt
                    .engines
                    .eval_param_checked(&app.graph, id, &key, &app.time)
                {
                    Ok(v) => (v, None),
                    Err(e) => (
                        rt.engines
                            .eval_param(&app.graph, id, &key, &app.time)
                            .unwrap_or_else(|| p.value.clone()),
                        Some(e),
                    ),
                }
            } else {
                (p.value.clone(), None)
            };
            let error = p.error().map(|e| e.to_string()).or(eval_err);
            (
                label,
                p.mode,
                p.value.clone(),
                p.expression.clone(),
                p.range,
                p.menu.clone(),
                error,
                evaluated,
            )
        };

        let ui_mode = match mode {
            ParamMode::Constant | ParamMode::Expression => mode,
            _ => ParamMode::Expression,
        };

        let mut changed = false;
        let mut new_mode = ui_mode;
        let button_value =
            is_panel && op_type == otd_engine::panel::BUTTON && key == "value";

        if is_panel {
            changed = crate::inspector::panel_param_row(
                ui,
                &label,
                &key,
                &mut value,
                range,
                menu.as_deref(),
                button_value,
            );
            if changed {
                app.edit(&format!("param:{}:{key}", app.graph.path(id)));
                let node = app.graph.node_mut(id);
                let p = node.params.get_mut(&key).unwrap();
                p.value = value;
                p.mode = ParamMode::Constant;
                p.recompile();
            }
            continue;
        }

        {
        let (_, dropped) = ui.dnd_drop_zone::<ChannelDrag, _>(egui::Frame::NONE, |ui| {
            ui.set_min_height(22.0);
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(140.0, 18.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        ui.add(egui::Label::new(&label).truncate());
                    },
                );

                let (glyph, tint) = match ui_mode {
                    ParamMode::Constant => ("=", Color32::from_rgb(140, 142, 150)),
                    ParamMode::Expression => ("ƒ", Color32::from_rgb(150, 200, 255)),
                    _ => ("ƒ", Color32::from_rgb(150, 200, 255)),
                };
                let btn = ui
                    .add(
                        egui::Button::new(RichText::new(glyph).color(tint))
                            .min_size([20.0, 18.0].into()),
                    )
                    .on_hover_text("Constant / Expression");
                if btn.clicked() {
                    new_mode = match ui_mode {
                        ParamMode::Constant => ParamMode::Expression,
                        ParamMode::Expression => ParamMode::Constant,
                        _ => ParamMode::Constant,
                    };
                    changed = true;
                }

                match ui_mode {
                    ParamMode::Expression => {
                        if !app.document.channel_expr_clipboard.is_empty()
                            && ui
                                .small_button("Apply")
                                .on_hover_text("Use copied channel expression")
                                .clicked()
                        {
                            expression = app.document.channel_expr_clipboard.clone();
                            changed = true;
                            new_mode = ParamMode::Expression;
                        }
                        if ui.text_edit_singleline(&mut expression).changed() {
                            changed = true;
                            new_mode = ParamMode::Expression;
                        }
                    }
                    ParamMode::Constant => {
                        changed |= match (&mut value, range) {
                            (Value::Float(_) | Value::Int(_), Some(_)) => {
                                crate::inspector::td_slider_field(ui, &key, &mut value, range)
                            }
                            _ if op_type == "selectCHOP" && key == "channames" => {
                                let names = select_source_channel_names(app, id);
                                chop_input_channel_picker(
                                    ui,
                                    &format!("{id:?}:channames"),
                                    &mut value,
                                    &names,
                                )
                            }
                            _ => crate::inspector::value_widget_basic(
                                ui,
                                &key,
                                &mut value,
                                range,
                                menu.as_deref(),
                            ),
                        };
                    }
                    ParamMode::Export | ParamMode::Bind => {}
                }
            });
        });

        if let Some(drag) = dropped {
            app.edit("expr");
            app.document.history.end_gesture();
            let expr = channel_expr_snippet(&drag.op_path, &drag.channel);
            app.document.channel_expr_clipboard = expr.clone();
            let p = app.graph.node_mut(id).params.get_mut(&key).unwrap();
            p.set_expression(&expr);
            app.document.status = format!("{label} <- {expr}");
            continue;
        }

        match ui_mode {
            ParamMode::Expression => {
                ui.horizontal(|ui| {
                    ui.add_space(126.0);
                    match &error {
                        Some(e) => {
                            ui.colored_label(
                                Color32::from_rgb(230, 120, 120),
                                RichText::new(e).small(),
                            );
                        }
                        None => {
                            ui.label(
                                RichText::new(format!("= {}", format_value(&evaluated)))
                                    .weak()
                                    .small(),
                            );
                        }
                    }
                });
            }
            ParamMode::Constant => {}
            _ => {}
        }
        }

        if changed {
            // The tag is the parameter itself, so a slider dragged across a
            // hundred frames is one undo entry and the next parameter touched
            // starts a new one.
            app.edit(&format!("param:{}:{key}", app.graph.path(id)));
            let node = app.graph.node_mut(id);
            let p = node.params.get_mut(&key).unwrap();
            match new_mode {
                ParamMode::Expression => {
                    p.expression = expression;
                    p.mode = ParamMode::Expression;
                    p.recompile();
                }
                _ => {
                    p.value = value;
                    p.mode = ParamMode::Constant;
                    p.recompile();
                }
            }
            if key == otd_dat::callbacks::PARAM_CALLBACKS {
                otd_dat::callbacks::sync_owner_callback_source(&mut app.graph, id);
                otd_dat::callbacks::reconcile_callback_links(&mut app.graph);
            }
        }
    }
    });
}
