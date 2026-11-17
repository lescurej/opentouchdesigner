use egui::{Pos2, Rect};
use otd_chop::ChopData;
use otd_core::NodeId;

use crate::app::OtdApp;
use crate::params::{ChannelDrag, channel_expr_snippet, copy_channel_expression};

use super::drag::channel_drag_grab_id;
use super::plot::*;

fn chop_tile_row_rects(body: Rect, zoom: f32, data: &ChopData) -> Vec<Rect> {
    let nc = data.channels.len();
    if nc == 0 {
        return Vec::new();
    }
    let ns = displayed_sample_count(data);
    if ns <= 1 {
        let row_h = body.height() / nc as f32;
        return (0..nc)
            .map(|ci| {
                Rect::from_min_max(
                    Pos2::new(body.min.x, body.min.y + ci as f32 * row_h),
                    Pos2::new(body.max.x, body.min.y + (ci as f32 + 1.0) * row_h),
                )
            })
            .collect();
    }
    let lanes = nc;
    let (_, _, plot) = chop_plot_layout(body, zoom, data.channels.len() <= 1);
    let lane_h = plot.height() / lanes as f32;
    (0..lanes)
        .map(|li| {
            Rect::from_min_max(
                Pos2::new(plot.min.x, plot.min.y + li as f32 * lane_h),
                Pos2::new(plot.max.x, plot.min.y + (li as f32 + 1.0) * lane_h),
            )
        })
        .collect()
}

pub(crate) fn chop_tile_channel_handles(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: NodeId,
    body: Rect,
    zoom: f32,
    viewer_operate: bool,
) -> (bool, bool) {
    if zoom < 0.32 {
        return (false, false);
    }
    let snapshot = {
        let rt = app.runtime().lock().expect("engine runtime");
        rt.engines.chop_data(id).map(|data| {
            let channel_indices: Vec<usize> = (0..data.channels.len()).collect();
            let names: Vec<String> = channel_indices
                .iter()
                .map(|&ci| data.channels[ci].name.clone())
                .collect();
            let preview_peak = if displayed_sample_count(data) <= 1 {
                chop_bipolar_peak(data, channel_indices.len(), 1)
            } else {
                let ns = displayed_sample_count(data);
                let (lo, hi) = chop_value_bounds_channels(data, &channel_indices, ns);
                lo.abs().max(hi.abs()).max(1e-6)
            };
            let preview_values: Vec<f32> = channel_indices
                .iter()
                .map(|&ci| {
                    data.channels[ci]
                        .samples
                        .first()
                        .copied()
                        .filter(|v| v.is_finite())
                        .unwrap_or(0.0)
                })
                .collect();
            let row_rects = chop_tile_row_rects(body, zoom, data);
            (
                channel_indices,
                names,
                preview_values,
                preview_peak,
                row_rects,
            )
        })
    };
    let Some((channel_indices, names, preview_values, preview_peak, row_rects)) = snapshot else {
        return (false, false);
    };
    if channel_indices.is_empty() || row_rects.len() != channel_indices.len() {
        return (false, false);
    }
    let path = app.graph.path(id);
    let cmd = ui.input(|i| i.modifiers.command);
    let pointer = ui.ctx().pointer_interact_pos();
    let mut blocks_node = false;
    let cmd_over_channel =
        cmd && pointer.is_some_and(|p| row_rects.iter().any(|row| row.contains(p)));
    for (row_i, (&ci, name)) in channel_indices.iter().zip(names.iter()).enumerate() {
        let row = row_rects[row_i];
        let snippet = channel_expr_snippet(&path, name);
        let payload = ChannelDrag {
            op_path: path.clone(),
            channel: name.to_string(),
            color_index: ci,
            preview_value: preview_values[row_i],
            preview_peak,
        };
        let drag_id = egui::Id::new(("tile_chan", id, ci));
        let channel_drag_active = ui.ctx().is_being_dragged(drag_id);
        if !cmd && !viewer_operate && !channel_drag_active {
            continue;
        }
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(row));
        let dnd = child.dnd_drag_source(drag_id, payload, |ui| {
            ui.set_min_size(row.size());
        });
        let response = dnd.response;
        if response.drag_started() {
            if let Some(p) = ui.ctx().pointer_interact_pos() {
                ui.ctx().data_mut(|d| {
                    d.insert_temp(channel_drag_grab_id(&path, name), p - row.min);
                });
            }
        }
        if response.double_clicked() {
            copy_channel_expression(app, ui.ctx(), snippet.clone());
        }
        if response.drag_started() || response.dragged() {
            blocks_node = true;
            if matches!(app.canvas.drag, Some(crate::canvas::DragState::Node { .. })) {
                app.canvas.drag = None;
            }
        }
        let hover = if viewer_operate {
            format!("{snippet}\nDrag onto a parameter")
        } else {
            format!("{snippet}\n⌘-drag onto a parameter")
        };
        response
            .on_hover_cursor(egui::CursorIcon::Grab)
            .on_hover_text(hover);
    }
    (blocks_node, cmd_over_channel)
}
