use egui::{Color32, RichText};
use otd_core::{ParamMode, Value};

use crate::app::OtdApp;

use super::{channel_expr_snippet, format_value};
use super::ChannelDrag;

pub(crate) fn constant_chop_row_count(app: &OtdApp, id: otd_core::NodeId) -> usize {
    app.graph
        .node(id)
        .param("rows")
        .map(|p| p.value.as_i64())
        .unwrap_or(1)
        .clamp(1, otd_chop::ops::CONSTANT_MAX_ROWS) as usize
}
pub(crate) struct ConstantChopRow {
    name: String,
    value: f32,
    mode: ParamMode,
    expression: String,
    preview: Option<Result<Value, String>>,
}
pub(crate) fn constant_chop_read_rows(app: &OtdApp, id: otd_core::NodeId, rows: usize) -> Vec<ConstantChopRow> {
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
                    app.engine_rt().engines
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
pub(crate) fn constant_chop_write_rows(
    app: &mut OtdApp,
    id: otd_core::NodeId,
    rows: &[ConstantChopRow],
    discrete: bool,
) {
    if discrete {
        app.document.history.end_gesture();
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
        app.document.history.end_gesture();
    }
}
pub(crate) fn constant_chop_rows(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
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
                            if !app.document.channel_expr_clipboard.is_empty()
                                && ui
                                    .small_button("Apply")
                                    .on_hover_text("Use copied channel expression")
                                    .clicked()
                            {
                                row_data[i].expression = app.document.channel_expr_clipboard.clone();
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
            app.document.history.end_gesture();
            let expr = channel_expr_snippet(&drag.op_path, &drag.channel);
            app.document.channel_expr_clipboard = expr.clone();
            let _ = app.graph.set_expression(id, &format!("value{i}"), &expr);
            app.document.status = format!("value{i} <- {expr}");
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
