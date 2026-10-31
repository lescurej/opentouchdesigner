//! The parameter panel.
//!
//! Parameter panel: Constant and Expression modes (TouchDesigner-style).
//! Legacy Export/Bind values in `.otd` files migrate to `ch()` expressions on open.

use egui::{Color32, RichText};
use otd_core::{ParamMode, Value};

use crate::app::OtdApp;

pub fn show(app: &mut OtdApp, ui: &mut egui::Ui) {
    ui.set_min_width(ui.available_width().max(480.0));
    let Some(id) = app.selected.filter(|s| app.graph.contains(*s)) else {
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
    } else if let Some(msg) = app.engines.node_message(&app.graph, id) {
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

    let pages = crate::inspector::collect_pages(&app.graph.node(id).params, is_panel);
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
        if !crate::inspector::param_on_page(&app.graph.node(id).params[&key], &active_page, has_pages)
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
                match app
                    .engines
                    .eval_param_checked(&app.graph, id, &key, &app.time)
                {
                    Ok(v) => (v, None),
                    Err(e) => (
                        app.engines
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
                        if !app.channel_expr_clipboard.is_empty()
                            && ui
                                .small_button("Apply")
                                .on_hover_text("Use copied channel expression")
                                .clicked()
                        {
                            expression = app.channel_expr_clipboard.clone();
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
                            _ if op_type == "selectCHOP" && key == "channels" => {
                                {
                                    let names = wired_input_chop_channel_names(app, id);
                                    chop_input_channel_picker(
                                        ui,
                                        &format!("{id:?}:channels"),
                                        &mut value,
                                        &names,
                                    )
                                }
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
        let dropped = dropped;

        if let Some(drag) = dropped {
            app.edit("expr");
            app.history.end_gesture();
            let expr = channel_expr_snippet(&drag.op_path, &drag.channel);
            app.channel_expr_clipboard = expr.clone();
            let p = app.graph.node_mut(id).params.get_mut(&key).unwrap();
            p.set_expression(&expr);
            app.status = format!("{label} <- {expr}");
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

/// Where a component's contents come from: a shared file, or another
/// component it tracks.
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

fn constant_chop_row_count(app: &OtdApp, id: otd_core::NodeId) -> usize {
    app.graph
        .node(id)
        .param("rows")
        .map(|p| p.value.as_i64())
        .unwrap_or(1)
        .clamp(1, otd_chop::ops::CONSTANT_MAX_ROWS) as usize
}

struct ConstantChopRow {
    name: String,
    value: f32,
    mode: ParamMode,
    expression: String,
    preview: Option<Result<Value, String>>,
}

fn constant_chop_read_rows(app: &OtdApp, id: otd_core::NodeId, rows: usize) -> Vec<ConstantChopRow> {
    (0..rows)
        .map(|i| {
            let name = app
                .graph
                .node(id)
                .param(&format!("name{i}"))
                .map(|p| p.value.as_str())
                .unwrap_or_else(|| "chan".to_string());
            let key = format!("value{i}");
            let (value, mode, expression) = app
                .graph
                .node(id)
                .param(&key)
                .map(|p| {
                    let mode = match p.mode {
                        ParamMode::Constant | ParamMode::Expression => p.mode,
                        _ => ParamMode::Expression,
                    };
                    (p.value.as_f32(), mode, p.expression.clone())
                })
                .unwrap_or((0.0, ParamMode::Constant, String::new()));
            let preview = if mode == ParamMode::Expression {
                Some(
                    app.engines
                        .eval_param_checked(&app.graph, id, &key, &app.time),
                )
            } else {
                None
            };
            ConstantChopRow {
                name,
                value,
                mode,
                expression,
                preview,
            }
        })
        .collect()
}

fn constant_chop_write_rows(
    app: &mut OtdApp,
    id: otd_core::NodeId,
    rows: &[ConstantChopRow],
    discrete: bool,
) {
    if discrete {
        app.history.end_gesture();
    }
    app.edit(&format!("constant rows:{}", app.graph.path(id)));
    let _ = app
        .graph
        .set_param(id, "rows", Value::Int(rows.len() as i64));
    for (i, row) in rows.iter().enumerate() {
        let _ = app.graph.set_param(
            id,
            &format!("name{i}"),
            Value::Str(row.name.clone()),
        );
        let key = format!("value{i}");
        if row.mode == ParamMode::Expression {
            let _ = app.graph.set_expression(id, &key, &row.expression);
        } else {
            let _ = app
                .graph
                .set_param(id, &key, Value::Float(row.value as f64));
        }
    }
    if discrete {
        app.history.end_gesture();
    }
}

fn constant_chop_rows(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let rows = constant_chop_row_count(app, id);
    let max = otd_chop::ops::CONSTANT_MAX_ROWS as usize;
    let mut move_row: Option<(usize, usize)> = None;
    let mut remove_row: Option<usize> = None;
    let mut add_row = false;

    ui.horizontal(|ui| {
        ui.strong("Name / value");
        if ui.button("+").on_hover_text("Add row").clicked() && rows < max {
            add_row = true;
        }
    });
    ui.label(
        RichText::new("Patterns like chan[1-4] share one value; reorder rows to reorder channel groups.")
            .weak()
            .small(),
    );

    let mut row_data = constant_chop_read_rows(app, id, rows);

    for i in 0..row_data.len() {
        let mut changed = false;
        let mut dropped = None;
        ui.horizontal(|ui| {
            if row_data.len() > 1 {
                ui.vertical(|ui| {
                    if i > 0 && ui.small_button("↑").clicked() {
                        move_row = Some((i, i - 1));
                    }
                    if i + 1 < row_data.len() && ui.small_button("↓").clicked() {
                        move_row = Some((i, i + 1));
                    }
                });
                if ui.small_button("✕").on_hover_text("Remove row").clicked() {
                    remove_row = Some(i);
                }
            }
            ui.label(RichText::new("Name").small().weak());
            changed |= ui
                .add(
                    egui::TextEdit::singleline(&mut row_data[i].name)
                        .desired_width(120.0),
                )
                .changed();
            ui.label(RichText::new("Value").small().weak());
            let (_, payload) = ui.dnd_drop_zone::<ChannelDrag, _>(egui::Frame::NONE, |ui| {
                ui.horizontal(|ui| {
                    let ui_mode = row_data[i].mode;
                    let (glyph, tint) = match ui_mode {
                        ParamMode::Expression => ("ƒ", Color32::from_rgb(150, 200, 255)),
                        _ => ("=", Color32::from_rgb(140, 142, 150)),
                    };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(glyph).color(tint))
                                .min_size([20.0, 18.0].into()),
                        )
                        .on_hover_text("Constant / Expression")
                        .clicked()
                    {
                        row_data[i].mode = match ui_mode {
                            ParamMode::Expression => ParamMode::Constant,
                            _ => ParamMode::Expression,
                        };
                        changed = true;
                    }
                    match ui_mode {
                        ParamMode::Expression => {
                            if !app.channel_expr_clipboard.is_empty()
                                && ui
                                    .small_button("Apply")
                                    .on_hover_text("Use copied channel expression")
                                    .clicked()
                            {
                                row_data[i].expression = app.channel_expr_clipboard.clone();
                                row_data[i].mode = ParamMode::Expression;
                                changed = true;
                            }
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut row_data[i].expression)
                                        .desired_width(180.0)
                                        .hint_text("absTime, ch('/op', 'chan')"),
                                )
                                .changed()
                            {
                                row_data[i].mode = ParamMode::Expression;
                                changed = true;
                            }
                        }
                        _ => {
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut row_data[i].value)
                                        .speed(0.01)
                                        .fixed_decimals(4),
                                )
                                .changed();
                        }
                    }
                });
            });
            dropped = payload;
        });
        if row_data[i].mode == ParamMode::Expression {
            if let Some(preview) = &row_data[i].preview {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    match preview {
                        Ok(v) => {
                            ui.label(
                                RichText::new(format!("= {}", format_value(v)))
                                    .weak()
                                    .small(),
                            );
                        }
                        Err(e) => {
                            ui.colored_label(
                                Color32::from_rgb(230, 120, 120),
                                RichText::new(e).small(),
                            );
                        }
                    }
                });
            }
        }
        if let Some(drag) = dropped {
            app.edit(&format!("constant rows:{}", app.graph.path(id)));
            app.history.end_gesture();
            let expr = channel_expr_snippet(&drag.op_path, &drag.channel);
            app.channel_expr_clipboard = expr.clone();
            let _ = app.graph.set_expression(id, &format!("value{i}"), &expr);
            app.status = format!("value{i} <- {expr}");
        } else if changed {
            constant_chop_write_rows(app, id, &row_data, false);
        }
    }

    if add_row {
        row_data.push(ConstantChopRow {
            name: "chan".into(),
            value: 0.0,
            mode: ParamMode::Constant,
            expression: String::new(),
            preview: None,
        });
        constant_chop_write_rows(app, id, &row_data, true);
    } else if let Some((from, to)) = move_row {
        row_data.swap(from, to);
        constant_chop_write_rows(app, id, &row_data, true);
    } else if let Some(rm) = remove_row {
        row_data.remove(rm);
        constant_chop_write_rows(app, id, &row_data, true);
    }
}

/// Add and remove a component's own parameters.
///
/// PLAN.md §2.3: "Custom parameters on components ARE the component API."
/// Operators inside read them as `parent.<name>`, so this panel is where a
/// network becomes a reusable thing with knobs.
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
                        app.history.end_gesture();
                        app.graph.add_custom_param(id, &name, param);
                        app.status = format!("added parent.{name}");
                        app.new_param_name.clear();
                    }
                }
            });
        });
}

/// Shared multiline DAT `text` field editor (parameter panel and node viewer).
pub(crate) fn show_dat_text_editor(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: otd_core::NodeId,
    rows: usize,
    text_area_id: egui::Id,
    interactive: bool,
) -> egui::Response {
    let mut text = app.graph.node(id).params["text"].value.as_str();
    let lint_id = crate::code_editor::dat_editor_id(id);
    let me = crate::code_lint::me_path_for_node(&app.graph, id);
    let label = format!("<{}>", app.graph.path(id));
    let is_python = dat_text_editor_is_python(app, id);
    let now = ui.input(|i| i.time);
    if is_python {
        app.code_lint.touch(lint_id, &text, now);
        app.code_lint
            .poll(lint_id, now, &app.engines, &app.graph, &me, &label);
    }
    let paths = crate::code_lint::operator_paths(&app.graph);
    let issues_owned = app.code_lint.issues(lint_id).to_vec();
    let language = if is_python { "python" } else { "text" };
    let extras = is_python.then(|| crate::code_editor::CodeEditorExtras {
        issues: &issues_owned,
        op_paths: &paths,
        me_path: &me,
        graph: &app.graph,
        engines: &app.engines,
    });
    let response = crate::code_editor::show(
        ui,
        &mut text,
        crate::code_editor::CodeEditorConfig {
            language,
            rows,
            id: text_area_id,
            interactive,
        },
        extras,
    );
    if response.changed() && interactive {
        app.edit(&format!("text:{id:?}"));
        let _ = app.graph.set_param(id, "text", Value::Str(text));
    }
    response
}

pub(crate) fn dat_text_editor_is_python(app: &OtdApp, id: otd_core::NodeId) -> bool {
    if crate::code_editor::is_callback_dat_text(app, id) {
        return true;
    }
    app.graph
        .node(id)
        .params
        .get("text")
        .is_some_and(|p| p.is_script())
}

pub(crate) fn dat_uses_text_param_editor(app: &OtdApp, id: otd_core::NodeId) -> bool {
    app.graph.node(id).params.contains_key("text")
        && app
            .engines
            .dat_data(id)
            .is_some_and(|d| d.is_text)
}

pub(crate) fn dat_node_viewer_editor(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: otd_core::NodeId,
    body: egui::Rect,
    zoom: f32,
    interactive: bool,
) {
    if zoom < 0.32 || body.width() < 48.0 || body.height() < 28.0 {
        return;
    }
    let body = if app.canvas_rect.is_positive() {
        body.intersect(app.canvas_rect)
    } else {
        body
    };
    if body.width() < 48.0 || body.height() < 28.0 {
        return;
    }
    let row_h = (14.0 * zoom).clamp(11.0, 18.0);
    let rows = ((body.height() / row_h).floor() as usize).clamp(4, 64);
    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
        ui.set_clip_rect(body);
        ui.set_width(body.width());
        show_dat_text_editor(
            app,
            ui,
            id,
            rows,
            crate::code_editor::dat_node_viewer_editor_id(id),
            interactive,
        );
    });
}

/// The multiline editor for a Table or Text DAT's contents.
fn text_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let rows = app
        .engines
        .dat_data(id)
        .map(|d| format!("{} × {}", d.num_rows(), d.num_cols()))
        .unwrap_or_default();

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Contents");
        ui.label(RichText::new(rows).weak().small());
    });

    let is_python = dat_text_editor_is_python(app, id);
    let _ = show_dat_text_editor(
        app,
        ui,
        id,
        if is_python { 18 } else { 14 },
        crate::code_editor::dat_editor_id(id),
        true,
    );
    if is_python {
        ui.label(
            RichText::new("Autocomplete in op('…') and ch('…', '…') as you type")
                .weak()
                .small(),
        );
        let editor_id = crate::code_editor::dat_editor_id(id);
        crate::code_lint::show_issues(ui, app.code_lint.issues(editor_id));
    }
    ui.add_space(4.0);
}

/// What a dragged channel carries. Dropping it on a parameter exports it.
#[derive(Clone, Debug)]
pub struct ChannelDrag {
    pub op_path: String,
    pub channel: String,
    pub color_index: usize,
    pub preview_value: f32,
    pub preview_peak: f32,
}

pub fn channel_expr_snippet(op_path: &str, channel: &str) -> String {
    format!("op('{op_path}')['{channel}']")
}

fn wired_input_chop_channel_names(app: &OtdApp, id: otd_core::NodeId) -> Vec<String> {
    let Some(src) = app
        .graph
        .node(id)
        .inputs
        .first()
        .and_then(|slot| *slot)
    else {
        return Vec::new();
    };
    app.engines
        .chop_data(src)
        .map(|data| data.channels.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default()
}

fn chop_input_channel_picker(
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

pub fn copy_channel_expression(app: &mut OtdApp, ctx: &egui::Context, snippet: String) {
    app.channel_expr_clipboard = snippet.clone();
    ctx.copy_text(snippet.clone());
    app.status = format!("copied {snippet}");
}

fn migrate_legacy_modes(app: &mut OtdApp, id: otd_core::NodeId) {
    let keys: Vec<String> = app.graph.node(id).params.keys().cloned().collect();
    for key in keys {
        let p = app.graph.node(id).params.get(&key).unwrap();
        let mode = p.mode;
        if mode == ParamMode::Export {
            if let Some((op, ch)) = p.source_parts() {
                let expr = channel_expr_snippet(&op, &ch);
                app.graph.node_mut(id).params.get_mut(&key).unwrap().set_expression(&expr);
            }
        } else if mode == ParamMode::Bind {
            if let Some((op, par)) = p.source_parts() {
                let expr = format!("par('{op}', '{par}')");
                app.graph.node_mut(id).params.get_mut(&key).unwrap().set_expression(&expr);
            }
        }
    }
}

/// Any parameter that names a file on disk: a text box you can paste into, a
/// Browse button for everyone else, and a line saying whether the file is
/// actually there.
///
/// Every file parameter gets this, not just the movie player's — a font that
/// silently does not load and a movie that silently does not load are the same
/// bug, and neither should need a trip to the terminal to diagnose.
fn file_row(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId, key: &str) {
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

fn script_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let mut source = app.graph.node(id).params["source"].value.as_str();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Source");
        ui.label(
            RichText::new("Python — set `rows` for Script DAT; `def onFrameStart` for Execute DATs")
                .weak()
                .small(),
        );
    });
    let editor_id = ui.id().with(("script", id));
    let me = app.graph.path(id);
    let label = format!("<{}>", me);
    let now = ui.input(|i| i.time);
    app.code_lint.touch(editor_id, &source, now);
    app.code_lint
        .poll(editor_id, now, &app.engines, &app.graph, &me, &label);
    let paths = crate::code_lint::operator_paths(&app.graph);
    let issues = app.code_lint.issues(editor_id);
    let response = crate::code_editor::show(
        ui,
        &mut source,
        crate::code_editor::CodeEditorConfig {
            language: "python",
            rows: 16,
            id: editor_id,
            interactive: true,
        },
        Some(crate::code_editor::CodeEditorExtras {
            issues,
            op_paths: &paths,
            me_path: &me,
            graph: &app.graph,
            engines: &app.engines,
        }),
    );
    if response.changed() {
        app.edit(&format!("script:{id:?}"));
        let _ = app.graph.set_param(id, "source", Value::Str(source));
    }
    crate::code_lint::show_issues(ui, app.code_lint.issues(editor_id));
    if let Some(msg) = app.engines.node_message(&app.graph, id) {
        ui.colored_label(
            crate::canvas::node_message::text_color(msg.level),
            RichText::new(&msg.text).small().monospace(),
        );
    }
    ui.add_space(4.0);
}

fn shader_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let mut source = app.graph.node(id).params["source"].value.as_str();
    let is_glsl = app
        .graph
        .node(id)
        .params
        .get("language")
        .map(|p| p.value.as_str() == "glsl")
        .unwrap_or(false);

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Source");
        ui.label(
            RichText::new(if is_glsl {
                "Shadertoy GLSL — write mainImage(); iTime, iResolution, iFrame are provided"
            } else {
                "WGSL fragment body — in.uv, U.time.x, U.res, sample0/1(uv), U.p0..p3"
            })
            .weak()
            .small(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Import ISF…")
                .on_hover_text(
                    "Load an ISF shader (.fs). Its inputs become parameters on this node.",
                )
                .clicked()
            {
                app.import_isf(id);
            }
        });
    });

    let lang = if is_glsl { "glsl" } else { "rs" };
    let editor_id = ui.id().with(("shader", id));
    let response = crate::code_editor::show(
        ui,
        &mut source,
        crate::code_editor::CodeEditorConfig {
            language: lang,
            rows: 16,
            id: editor_id,
            interactive: true,
        },
        None,
    );
    if response.changed() {
        app.edit(&format!("shader:{id:?}"));
        let _ = app.graph.set_param(id, "source", Value::Str(source));
    }

    // The compile error comes from the GPU engine, not the parameter, because
    // the shader is only compiled when the node cooks.
    match app.engines.top.shader_error(id) {
        Some(err) => {
            ui.colored_label(
                Color32::from_rgb(235, 120, 120),
                RichText::new(err).small().monospace(),
            );
            ui.label(
                RichText::new("holding the last shader that compiled")
                    .weak()
                    .small(),
            );
        }
        None => {
            ui.colored_label(
                Color32::from_rgb(130, 200, 140),
                RichText::new("compiled").small(),
            );
        }
    }
    ui.add_space(4.0);
}

fn format_value(v: &Value) -> String {
    match v {
        Value::Float(f) => format!("{f:.4}"),
        other => other.as_str(),
    }
}

/// The keyframe editor for an Animation CHOP.
///
/// Keys are stored as text and that text stays editable below — this draws the
/// same data as a curve because a shape is not something you read as numbers.
/// Both directions write through `Curves`, so the two views cannot disagree.
fn curve_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    use otd_chop::anim::{Curves, Interp};

    let text = app.graph.node(id).params["keys"].value.as_str();
    let (mut curves, problems) = Curves::parse(&text);

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Keys");
        ui.label(
            RichText::new("click to add · drag to move · shift-click to delete")
                .weak()
                .small(),
        );
    });

    // The horizontal axis is the timeline's loop range, so the curve editor
    // and the scrub bar below it line up and the playhead means the same thing
    // in both.
    let (t0, t1) = app.loop_range;
    let span = (t1 - t0).max(1e-6);
    let (lo, hi) = value_range(&curves);

    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 120.0),
        egui::Sense::click_and_drag(),
    );
    let to_screen = |t: f64, v: f32| {
        egui::pos2(
            rect.left() + ((t - t0) / span) as f32 * rect.width(),
            rect.bottom() - ((v - lo) / (hi - lo)) * rect.height(),
        )
    };
    let from_screen = |p: egui::Pos2| {
        (
            t0 + ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * span,
            lo + (1.0 - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0)) * (hi - lo),
        )
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, Color32::from_gray(26));
    // The zero line, when it is in view — the reference nearly every curve is
    // read against.
    if lo < 0.0 && hi > 0.0 {
        let y = to_screen(t0, 0.0).y;
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, Color32::from_gray(52)),
        );
    }

    let mut changed = false;
    let names: Vec<String> = curves.0.keys().cloned().collect();
    for (n, name) in names.iter().enumerate() {
        let tint = channel_color(n);
        let curve = &curves.0[name];

        // Sample per pixel rather than drawing straight lines between keys:
        // an eased or splined segment is a curve, and drawing it as a chord
        // would show a shape the render does not have.
        let steps = rect.width().max(2.0) as usize;
        let points: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let t = t0 + span * (i as f64 / steps as f64);
                to_screen(t, curve.sample(t))
            })
            .collect();
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, tint)));

        for key in &curve.keys {
            painter.circle_filled(to_screen(key.time, key.value), 3.5, tint);
        }
    }

    // Editing acts on the channel the panel is focused on, which is the first
    // one unless the node has none yet.
    let target = names.first().cloned().unwrap_or_else(|| "chan1".into());

    if let Some(pos) = resp.interact_pointer_pos() {
        let (t, v) = from_screen(pos);
        let shift = ui.ctx().input(|i| i.modifiers.shift);
        // The nearest key within grabbing distance, in screen space so the
        // tolerance is the same however the axes are scaled.
        let hit = names.iter().find_map(|name| {
            curves.0[name]
                .keys
                .iter()
                .find(|k| to_screen(k.time, k.value).distance(pos) < 8.0)
                .map(|k| (name.clone(), k.time, k.interp))
        });

        if resp.drag_started() || resp.clicked() {
            match (&hit, shift) {
                (Some((name, time, _)), true) => {
                    app.edit("delete key");
                    app.history.end_gesture();
                    curves.0.get_mut(name).unwrap().remove_at(*time);
                    changed = true;
                }
                (None, false) => {
                    app.edit("add key");
                    app.history.end_gesture();
                    curves
                        .0
                        .entry(target.clone())
                        .or_default()
                        .set(t, v, Interp::Smooth);
                    changed = true;
                }
                _ => {}
            }
        } else if resp.dragged() {
            if let Some((name, time, interp)) = hit {
                // One undo entry for the whole drag: the tag names the key.
                app.edit(&format!("key:{name}:{time}"));
                let curve = curves.0.get_mut(&name).unwrap();
                curve.remove_at(time);
                curve.set(t, v, interp);
                changed = true;
            }
        }
    }
    if resp.drag_stopped() {
        app.history.end_gesture();
    }

    // The playhead, so a key can be placed against what is on screen.
    let x = to_screen(app.time.time, 0.0).x;
    if rect.x_range().contains(x) {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, Color32::from_rgb(230, 170, 90)),
        );
    }

    if changed {
        let _ = app
            .graph
            .set_param(id, "keys", Value::Str(curves.to_text()));
    }

    ui.horizontal(|ui| {
        for (n, name) in names.iter().enumerate() {
            ui.colored_label(channel_color(n), RichText::new(name).small());
        }
        ui.label(
            RichText::new(format!("{:.2} … {:.2}", lo, hi))
                .weak()
                .small(),
        );
    });

    // A hand-typed key with a typo is skipped rather than fatal, so the panel
    // has to say which line was dropped — otherwise it silently does nothing.
    for problem in &problems {
        ui.colored_label(
            Color32::from_rgb(235, 170, 120),
            RichText::new(problem).small(),
        );
    }

    let mut text = text;
    if ui
        .add(
            egui::TextEdit::multiline(&mut text)
                .code_editor()
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        app.edit(&format!("keys:{id:?}"));
        let _ = app.graph.set_param(id, "keys", Value::Str(text));
    }
    ui.add_space(4.0);
}

/// The vertical extent to draw, with a little air above and below.
fn value_range(curves: &otd_chop::anim::Curves) -> (f32, f32) {
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for curve in curves.0.values() {
        for key in &curve.keys {
            lo = lo.min(key.value);
            hi = hi.max(key.value);
        }
    }
    if lo > hi {
        // No keys yet. 0..1 is the range most parameters live in, so a first
        // click lands somewhere useful.
        return (0.0, 1.0);
    }
    // A flat curve has no extent of its own; give it one rather than dividing
    // by zero and drawing a line off the top of the box.
    let pad = ((hi - lo) * 0.15).max(0.05);
    (lo - pad, hi + pad)
}

fn channel_color(n: usize) -> Color32 {
    const PALETTE: [Color32; 6] = [
        Color32::from_rgb(150, 200, 255),
        Color32::from_rgb(150, 220, 150),
        Color32::from_rgb(230, 170, 90),
        Color32::from_rgb(220, 140, 200),
        Color32::from_rgb(200, 200, 130),
        Color32::from_rgb(160, 160, 190),
    ];
    PALETTE[n % PALETTE.len()]
}
