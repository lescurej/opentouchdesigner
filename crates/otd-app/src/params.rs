//! The parameter panel.
//!
//! Parameter panel: Constant and Expression modes (TouchDesigner-style).
//! Legacy Export/Bind values in `.otd` files migrate to `ch()` expressions on open.

use egui::{Color32, RichText};
use otd_core::{ParamMode, Value};

use crate::app::OtdApp;

pub fn show(app: &mut OtdApp, ui: &mut egui::Ui) {
    let Some(id) = app.selected.filter(|s| app.graph.contains(*s)) else {
        ui.label(RichText::new("No operator selected").weak());
        ui.add_space(8.0);
        ui.label(RichText::new("Tab / double-click network  add operator").weak().small());
        ui.label(RichText::new("Viewer flag on node  network preview").weak().small());
        ui.label(
            RichText::new("V D L B  flags on selection    Space  play/pause")
                .weak()
                .small(),
        );
        ui.label(RichText::new("H  reset view    Del  delete").weak().small());
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
    } else if let Some(status) = app.engines.node_status(&app.graph, id) {
        ui.label(
            RichText::new(status)
                .small()
                .color(Color32::from_rgb(230, 170, 110)),
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
        let mut channel_sources: Vec<(otd_core::NodeId, &str)> = Vec::new();
        if app.graph.node(id).family == otd_core::Family::Chop {
            channel_sources.push((id, "Selected operator"));
        }
        if let Some(vid) = app.perform_ui() {
            if app.graph.node(vid).family == otd_core::Family::Chop
                && Some(vid) != app.selected
            {
                channel_sources.push((vid, "Perform UI"));
            }
        }
        for (src, title) in channel_sources {
            channel_list(app, ui, src, title);
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

fn constant_chop_read_rows(app: &OtdApp, id: otd_core::NodeId, rows: usize) -> (Vec<String>, Vec<f32>) {
    let mut names = Vec::with_capacity(rows);
    let mut values = Vec::with_capacity(rows);
    for i in 0..rows {
        let name = app
            .graph
            .node(id)
            .param(&format!("name{i}"))
            .map(|p| p.value.as_str())
            .unwrap_or_else(|| "chan".to_string());
        let val = app
            .graph
            .node(id)
            .param(&format!("value{i}"))
            .map(|p| p.value.as_f32())
            .unwrap_or(0.0);
        names.push(name);
        values.push(val);
    }
    (names, values)
}

fn constant_chop_write_rows(
    app: &mut OtdApp,
    id: otd_core::NodeId,
    rows: usize,
    names: &[String],
    values: &[f32],
) {
    app.edit(&format!("constant rows:{}", app.graph.path(id)));
    let _ = app
        .graph
        .set_param(id, "rows", Value::Int(rows as i64));
    for i in 0..rows {
        let _ = app.graph.set_param(
            id,
            &format!("name{i}"),
            Value::Str(names[i].clone()),
        );
        let _ = app
            .graph
            .set_param(id, &format!("value{i}"), Value::Float(values[i] as f64));
    }
    app.history.end_gesture();
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

    let (mut names, mut values) = constant_chop_read_rows(app, id, rows);

    for i in 0..rows {
        ui.horizontal(|ui| {
            if rows > 1 {
                ui.vertical(|ui| {
                    if i > 0 && ui.small_button("↑").clicked() {
                        move_row = Some((i, i - 1));
                    }
                    if i + 1 < rows && ui.small_button("↓").clicked() {
                        move_row = Some((i, i + 1));
                    }
                });
                if ui.small_button("✕").on_hover_text("Remove row").clicked() {
                    remove_row = Some(i);
                }
            }
            ui.label(RichText::new("Name").small().weak());
            let name_changed = ui
                .add(
                    egui::TextEdit::singleline(&mut names[i])
                        .desired_width(120.0),
                )
                .changed();
            ui.label(RichText::new("Value").small().weak());
            let val_changed = ui
                .add(
                    egui::DragValue::new(&mut values[i])
                        .speed(0.01)
                        .fixed_decimals(4),
                )
                .changed();
            if name_changed || val_changed {
                constant_chop_write_rows(app, id, rows, &names, &values);
            }
        });
    }

    if add_row {
        let mut names = names;
        let mut values = values;
        names.push("chan".into());
        values.push(0.0);
        constant_chop_write_rows(app, id, rows + 1, &names, &values);
    } else if let Some((from, to)) = move_row {
        names.swap(from, to);
        values.swap(from, to);
        constant_chop_write_rows(app, id, rows, &names, &values);
    } else if let Some(rm) = remove_row {
        names.remove(rm);
        values.remove(rm);
        constant_chop_write_rows(app, id, rows - 1, &names, &values);
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
                app.edit("remove parameter");
                app.history.end_gesture();
                app.graph.remove_custom_param(id, &key);
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

/// The multiline editor for a Table or Text DAT's contents.
fn text_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let mut text = app.graph.node(id).params["text"].value.as_str();
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
    let response = ui.add(
        egui::TextEdit::multiline(&mut text)
            .code_editor()
            .desired_rows(10)
            .desired_width(f32::INFINITY),
    );
    if response.changed() {
        // Typing coalesces into one entry per editing session; clicking away
        // and coming back starts another.
        app.edit(&format!("text:{id:?}"));
        let _ = app.graph.set_param(id, "text", Value::Str(text));
    }
    ui.add_space(4.0);
}

/// What a dragged channel carries. Dropping it on a parameter exports it.
#[derive(Clone, Debug)]
pub struct ChannelDrag {
    pub op_path: String,
    pub channel: String,
}

pub fn channel_expr_snippet(op_path: &str, channel: &str) -> String {
    format!("op('{op_path}')['{channel}']")
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

/// The selected CHOP's channels, with their current values. Each row is a
/// drag source: drop one on a parameter to export it, which is the gesture
/// PLAN.md §5 calls for in Phase 2.
fn channel_list(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId, section: &str) {
    if app.graph.node(id).family != otd_core::Family::Chop {
        return;
    }
    let path = app.graph.path(id);
    let Some(data) = app.engines.chop_data(id) else {
        return;
    };
    let rows: Vec<(usize, String, f32, usize)> = data
        .channels
        .iter()
        .enumerate()
        .map(|(i, ch)| (i, ch.name.clone(), ch.last(), ch.samples.len()))
        .collect();
    let rate = data.sample_rate;

    ui.separator();
    ui.horizontal(|ui| {
        ui.strong("Channels");
        ui.label(RichText::new(section).weak().small());
        ui.label(
            RichText::new(format!("{} @ {rate:.0} Hz", rows.len()))
                .weak()
                .small(),
        );
    });
    if rows.is_empty() {
        ui.label(RichText::new("none").weak().small());
        return;
    }
    ui.label(
        RichText::new("Copy · double-click row · drag onto a ƒ parameter (select target op first)")
            .weak()
            .small(),
    );

    for (i, name, value, samples) in rows {
        let payload = ChannelDrag {
            op_path: path.clone(),
            channel: name.clone(),
        };
        let snippet = channel_expr_snippet(&path, &name);
        let mut row_response = None;
        ui.horizontal(|ui| {
            let row = ui.dnd_drag_source(egui::Id::new(("chan", id, i)), payload, |ui| {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 2.0, crate::canvas::channel_color(i));
                    ui.add_sized(
                        [110.0, 16.0],
                        egui::Label::new(RichText::new(&name).monospace()).truncate(),
                    );
                    ui.label(RichText::new(format!("{value:+.4}")).monospace());
                    if samples > 1 {
                        ui.label(RichText::new(format!("×{samples}")).weak().small());
                    }
                });
            });
            row_response = Some(row.response);
            if ui
                .small_button("Copy")
                .on_hover_text(&snippet)
                .clicked()
            {
                copy_channel_expression(app, ui.ctx(), snippet.clone());
            }
        });
        if let Some(r) = row_response {
            if r.double_clicked() {
                copy_channel_expression(app, ui.ctx(), snippet.clone());
            }
            r.context_menu(|ui| {
                if ui.button("Copy expression").clicked() {
                    copy_channel_expression(app, ui.ctx(), snippet.clone());
                    ui.close();
                }
            });
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
    let response = ui.add(
        egui::TextEdit::multiline(&mut source)
            .code_editor()
            .desired_rows(16)
            .desired_width(f32::INFINITY),
    );
    if response.changed() {
        app.edit(&format!("script:{id:?}"));
        let _ = app.graph.set_param(id, "source", Value::Str(source));
    }
    if let Some(err) = app.engines.node_status(&app.graph, id) {
        ui.colored_label(
            Color32::from_rgb(235, 120, 120),
            RichText::new(err).small().monospace(),
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

    let response = ui.add(
        egui::TextEdit::multiline(&mut source)
            .code_editor()
            .desired_rows(16)
            .desired_width(f32::INFINITY),
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
