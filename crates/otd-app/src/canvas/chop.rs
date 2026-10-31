use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use otd_chop::ChopData;
use otd_core::NodeId;

use crate::app::OtdApp;
use crate::params::{channel_expr_snippet, copy_channel_expression, ChannelDrag};

use super::palette::channel_color;
use super::types::{BODY_H, FOOTER_H, NODE_H, NODE_W};

const CHOP_LANE_H: f32 = BODY_H / 8.0;
const CHOP_MULTISAMPLE_AXIS_H: f32 = 14.0;

pub(crate) fn chop_node_world_height(data: &ChopData) -> f32 {
    let nc = data.channels.len();
    if nc == 0 {
        return NODE_H;
    }
    let axis = if displayed_sample_count(data) > 1 {
        CHOP_MULTISAMPLE_AXIS_H
    } else {
        0.0
    };
    let body = (nc as f32 * CHOP_LANE_H + axis).max(BODY_H);
    body + FOOTER_H
}

pub(crate) fn node_world_size(app: &OtdApp, id: NodeId) -> Vec2 {
    let main_h = super::callback_embed::main_node_world_height(app, id);
    let extra = super::callback_embed::callback_extra_height(app, id);
    Vec2::new(NODE_W, main_h + extra)
}

fn chop_axis_font(zoom: f32) -> FontId {
    FontId::proportional((8.0 * zoom).clamp(7.0, 11.0))
}

fn chop_lane_label_font(zoom: f32) -> FontId {
    FontId::proportional((11.0 * zoom).clamp(9.0, 16.0))
}

fn format_channel_value(v: f32) -> String {
    format_axis_number(v)
}

fn paint_chop_text(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: impl AsRef<str>,
    font: FontId,
    fg: Color32,
) {
    let text = text.as_ref();
    let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, 200);
    for (dx, dy) in [(1.0, 1.0), (1.0, 0.0), (0.0, 1.0)] {
        painter.text(
            pos + Vec2::new(dx, dy),
            align,
            text,
            font.clone(),
            shadow,
        );
    }
    painter.text(pos, align, text, font, fg);
}

fn format_axis_number(v: f32) -> String {
    if !v.is_finite() {
        return "?".into();
    }
    let av = v.abs();
    if av >= 1000.0 || (av > 0.0 && av < 0.001) {
        format!("{v:.2e}")
    } else if av >= 100.0 {
        format!("{v:.0}")
    } else if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i64)
    } else if av <= 2.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

fn format_x_sample_index(i: usize) -> String {
    format!("{i}i")
}

fn nice_step(raw: f32) -> f32 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f32.powf(exp);
    let frac = raw / base;
    let nice_frac = if frac <= 1.0 {
        1.0
    } else if frac <= 2.0 {
        2.0
    } else if frac <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice_frac * base
}

fn nice_value_ticks(lo: f32, hi: f32, max_ticks: usize) -> Vec<f32> {
    if !lo.is_finite() || !hi.is_finite() {
        return vec![0.0];
    }
    let span = (hi - lo).max(1e-12);
    let step = nice_step(span / max_ticks.max(2) as f32);
    let start = (lo / step).floor() * step;
    let mut ticks = Vec::new();
    let mut v = start;
    let end = hi + step * 0.001;
    while v <= end {
        if v >= lo - step * 0.001 && v <= hi + step * 0.001 {
            ticks.push(v);
        }
        v += step;
        if ticks.len() > max_ticks + 2 {
            break;
        }
    }
    if ticks.is_empty() {
        ticks.push(lo);
        if (hi - lo).abs() > 1e-6 {
            ticks.push(hi);
        }
    }
    ticks.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    ticks.dedup_by(|a, b| (*a - *b).abs() < step * 0.01);
    ticks
}

fn sample_index_ticks(ns: usize, max_ticks: usize) -> Vec<usize> {
    if ns == 0 {
        return Vec::new();
    }
    let last = ns.saturating_sub(1);
    if last == 0 {
        return vec![0];
    }
    let step_f = nice_step(last as f32 / max_ticks.max(2) as f32);
    let step = step_f.round().max(1.0) as usize;
    let mut out = Vec::new();
    let mut i = 0;
    while i <= last {
        out.push(i);
        i = i.saturating_add(step);
        if step == 0 {
            break;
        }
    }
    if out.last() != Some(&last) {
        out.push(last);
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn x_plot_x(plot: Rect, index: usize, last: usize) -> f32 {
    let t = index as f32 / last.max(1) as f32;
    plot.min.x + t * plot.width()
}

fn chop_plot_layout(body: Rect, zoom: f32) -> (Rect, Rect, Rect) {
    let left = (28.0 * zoom).clamp(22.0, 36.0);
    let bottom = (12.0 * zoom).clamp(10.0, 16.0);
    let y_axis = Rect::from_min_max(body.min, Pos2::new(body.min.x + left, body.max.y - bottom));
    let x_axis = Rect::from_min_max(
        Pos2::new(body.min.x + left, body.max.y - bottom),
        body.max,
    );
    let plot = Rect::from_min_max(
        Pos2::new(body.min.x + left, body.min.y),
        Pos2::new(body.max.x, body.max.y - bottom),
    );
    (y_axis, x_axis, plot)
}

fn draw_chop_plot_grid(
    painter: &egui::Painter,
    plot: Rect,
    y_ticks: &[f32],
    x_indices: &[usize],
    last: usize,
    lo: f32,
    hi: f32,
) {
    let grid = Color32::from_rgb(48, 50, 56);
    let span = (hi - lo).max(1e-12);
    for &v in y_ticks {
        let t = ((v - lo) / span).clamp(0.0, 1.0);
        let y = plot.max.y - t * plot.height();
        painter.line_segment(
            [Pos2::new(plot.min.x, y), Pos2::new(plot.max.x, y)],
            Stroke::new(1.0, grid),
        );
    }
    for &i in x_indices {
        let x = x_plot_x(plot, i, last);
        painter.line_segment(
            [Pos2::new(x, plot.min.y), Pos2::new(x, plot.max.y)],
            Stroke::new(1.0, grid),
        );
    }
}

fn draw_chop_y_axis(
    painter: &egui::Painter,
    axis: Rect,
    plot: Rect,
    lo: f32,
    hi: f32,
    zoom: f32,
) {
    let font = chop_axis_font(zoom);
    let color = Color32::from_rgb(130, 132, 140);
    let tick = Color32::from_rgb(58, 60, 66);
    let values = nice_value_ticks(lo, hi, 5);
    let span = (hi - lo).max(1e-12);
    for v in values {
        let t = ((v - lo) / span).clamp(0.0, 1.0);
        let y = plot.min.y + (1.0 - t) * plot.height();
        painter.line_segment(
            [Pos2::new(axis.max.x - 3.0, y), Pos2::new(axis.max.x, y)],
            Stroke::new(1.0, tick),
        );
        painter.text(
            Pos2::new(axis.max.x - 4.0, y),
            Align2::RIGHT_CENTER,
            format_axis_number(v),
            font.clone(),
            color,
        );
    }
}

fn draw_chop_x_axis(
    painter: &egui::Painter,
    axis: Rect,
    plot: Rect,
    ns: usize,
    zoom: f32,
) {
    if ns == 0 {
        return;
    }
    let font = chop_axis_font(zoom);
    let color = Color32::from_rgb(130, 132, 140);
    let tick = Color32::from_rgb(58, 60, 66);
    let last = ns.saturating_sub(1);
    let max_ticks = ((plot.width() / (36.0 * zoom).clamp(28.0, 44.0)).floor() as usize).clamp(3, 8);
    let indices = sample_index_ticks(ns, max_ticks);
    for i in indices {
        if i >= ns {
            continue;
        }
        let x = x_plot_x(plot, i, last);
        painter.line_segment(
            [Pos2::new(x, axis.min.y), Pos2::new(x, axis.min.y + 3.0)],
            Stroke::new(1.0, tick),
        );
        let label = format_x_sample_index(i);
        let align = if i == 0 {
            Align2::LEFT_TOP
        } else if i >= last {
            Align2::RIGHT_TOP
        } else {
            Align2::CENTER_TOP
        };
        painter.text(
            Pos2::new(x, axis.min.y + 2.0),
            align,
            label,
            font.clone(),
            color,
        );
    }
}

fn displayed_sample_count(data: &ChopData) -> usize {
    data.num_samples()
}

fn chop_value_bounds_channels(
    data: &ChopData,
    channel_indices: &[usize],
    sample_count: usize,
) -> (f32, f32) {
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    let n = sample_count.min(displayed_sample_count(data));
    for &ci in channel_indices {
        let Some(ch) = data.channels.get(ci) else {
            continue;
        };
        for &v in ch.samples.iter().take(n) {
            if v.is_finite() {
                min_v = min_v.min(v);
                max_v = max_v.max(v);
            }
        }
    }
    if !min_v.is_finite() {
        return (-1.0, 1.0);
    }
    if (max_v - min_v).abs() < 1e-12 {
        let c = min_v;
        let pad = c.abs().max(1.0) * 0.05 + 0.01;
        return (c - pad, c + pad);
    }
    let pad = (max_v - min_v) * 0.08;
    (min_v - pad, max_v + pad)
}

fn chop_bipolar_peak(data: &ChopData, channels: usize, samples: usize) -> f32 {
    let indices: Vec<usize> = (0..channels.min(data.channels.len())).collect();
    let (lo, hi) = chop_value_bounds_channels(data, &indices, samples);
    lo.abs().max(hi.abs()).max(1e-6)
}

pub(crate) fn channel_drag_grab_id(op_path: &str, channel: &str) -> egui::Id {
    egui::Id::new(("chan_drag_grab", op_path, channel))
}

fn channel_drag_ghost_size(zoom: f32) -> Vec2 {
    Vec2::new((132.0 * zoom).clamp(108.0, 168.0), (20.0 * zoom).clamp(18.0, 26.0))
}

fn draw_channel_drag_ghost(
    painter: &egui::Painter,
    rect: Rect,
    drag: &ChannelDrag,
    zoom: f32,
) {
    let stroke = Stroke::new(1.0, Color32::from_rgb(100, 160, 230));
    painter.rect_filled(rect, 3.0, Color32::from_rgb(32, 34, 40));
    painter.rect_stroke(rect, 3.0, stroke, egui::StrokeKind::Outside);
    let inner = rect.shrink(2.0);
    chop_bar_cell(
        painter,
        inner,
        drag.preview_value,
        drag.preview_peak.max(1e-6),
        zoom,
        channel_color(drag.color_index),
        true,
        Some(&drag.channel),
    );
}

pub(crate) fn paint_channel_drag_ghost(ctx: &egui::Context, zoom: f32) {
    let Some(drag) = egui::DragAndDrop::payload::<ChannelDrag>(ctx) else {
        return;
    };
    let Some(pointer) = ctx.pointer_interact_pos() else {
        return;
    };
    let size = channel_drag_ghost_size(zoom);
    let grab_id = channel_drag_grab_id(&drag.op_path, &drag.channel);
    let grab = ctx
        .data(|d| d.get_temp::<Vec2>(grab_id))
        .unwrap_or(size * 0.5);
    let rect = Rect::from_min_size(pointer - grab, size);
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("channel_drag_ghost"));
    let painter = ctx.layer_painter(layer);
    draw_channel_drag_ghost(&painter, rect, &drag, zoom);
}

fn chop_bar_cell(
    painter: &egui::Painter,
    cell: Rect,
    v: f32,
    peak: f32,
    zoom: f32,
    bar_color: Color32,
    show_value: bool,
    show_name: Option<&str>,
) {
    if cell.width() < 2.0 || cell.height() < 2.0 {
        return;
    }
    painter.rect_filled(cell, 0.0, Color32::from_rgb(36, 38, 44));
    let font = chop_lane_label_font(zoom);
    let name_color = Color32::from_rgb(248, 248, 252);
    let value_color = Color32::from_rgb(255, 220, 120);
    let name_w = (58.0 * zoom).clamp(44.0, 96.0);
    let val_w = (46.0 * zoom).clamp(36.0, 80.0);
    let bar = Rect::from_min_max(
        Pos2::new(cell.min.x + name_w, cell.min.y + 1.0),
        Pos2::new(cell.max.x - val_w, cell.max.y - 1.0),
    );
    if bar.width() > 4.0 {
        let mid_x = bar.center().x;
        painter.line_segment(
            [Pos2::new(mid_x, bar.min.y), Pos2::new(mid_x, bar.max.y)],
            Stroke::new(1.0, Color32::from_rgb(58, 60, 66)),
        );
        let half_w = bar.width() * 0.5;
        let norm = (v / peak).clamp(-1.0, 1.0);
        let x0 = mid_x + norm.min(0.0) * half_w;
        let x1 = mid_x + norm.max(0.0) * half_w;
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x0.min(x1), bar.min.y),
                Pos2::new(x0.max(x1), bar.max.y),
            ),
            0.0,
            bar_color,
        );
    }
    if cell.height() >= 7.0 {
        if let Some(name) = show_name {
            let short: String = name.chars().take(18).collect();
            paint_chop_text(
                painter,
                Pos2::new(cell.min.x + 3.0 * zoom, cell.center().y),
                Align2::LEFT_CENTER,
                short,
                font.clone(),
                name_color,
            );
        }
        if show_value {
            paint_chop_text(
                painter,
                Pos2::new(cell.max.x - 3.0 * zoom, cell.center().y),
                Align2::RIGHT_CENTER,
                format_channel_value(v),
                font,
                value_color,
            );
        }
    }
}

fn draw_chop_single_sample(
    painter: &egui::Painter,
    data: &ChopData,
    body: Rect,
    zoom: f32,
) {
    let nc = data.channels.len();
    if nc == 0 {
        return;
    }
    let peak = chop_bipolar_peak(data, nc, 1);
    let row_h = body.height() / nc as f32;
    let divider = Color32::from_rgb(44, 46, 52);

    for (ci, ch) in data.channels.iter().enumerate() {
        let row = Rect::from_min_max(
            Pos2::new(body.min.x, body.min.y + ci as f32 * row_h),
            Pos2::new(body.max.x, body.min.y + (ci as f32 + 1.0) * row_h),
        );
        if ci > 0 {
            painter.line_segment(
                [row.left_top(), row.right_top()],
                Stroke::new(1.0, divider),
            );
        }
        let v = ch.samples.first().copied().unwrap_or(0.0);
        chop_bar_cell(
            painter,
            row,
            v,
            peak,
            zoom,
            channel_color(ci),
            true,
            Some(&ch.name),
        );
    }
}

fn waveform_points(
    samples: &[f32],
    plot: Rect,
    lo: f32,
    hi: f32,
) -> Vec<Pos2> {
    let n = samples.len();
    if n == 0 {
        return Vec::new();
    }
    let span = (hi - lo).max(1e-12);
    let y_of = |v: f32| plot.max.y - ((v - lo) / span).clamp(0.0, 1.0) * plot.height();
    if n == 1 {
        return vec![Pos2::new(plot.center().x, y_of(samples[0]))];
    }
    let px = plot.width().ceil().max(2.0) as usize;
    if n <= px {
        let step = plot.width() / (n - 1) as f32;
        return samples
            .iter()
            .enumerate()
            .map(|(i, v)| Pos2::new(plot.min.x + i as f32 * step, y_of(*v)))
            .collect();
    }
    (0..px)
        .map(|pi| {
            let i = pi * (n - 1) / px.saturating_sub(1).max(1);
            Pos2::new(
                plot.min.x + pi as f32 * plot.width() / (px - 1).max(1) as f32,
                y_of(samples[i]),
            )
        })
        .collect()
}

fn draw_chop_multisample(
    painter: &egui::Painter,
    data: &ChopData,
    body: Rect,
    zoom: f32,
) {
    let lanes: Vec<_> = data.channels.iter().enumerate().collect();
    if lanes.is_empty() {
        return;
    }
    let ns = displayed_sample_count(data);
    if ns < 2 {
        return;
    }
    let lane_indices: Vec<usize> = lanes.iter().map(|(ci, _)| *ci).collect();
    let (lo, hi) = chop_value_bounds_channels(data, &lane_indices, ns);
    let (y_axis, x_axis, plot_area) = chop_plot_layout(body, zoom);
    painter.rect_filled(plot_area, 0.0, Color32::from_rgb(18, 19, 22));
    let last = ns.saturating_sub(1);
    let max_x_ticks = ((plot_area.width() / (36.0 * zoom).clamp(28.0, 44.0)).floor() as usize)
        .clamp(3, 8);
    let x_indices = sample_index_ticks(ns, max_x_ticks);
    let y_ticks = nice_value_ticks(lo, hi, 5);
    draw_chop_plot_grid(painter, plot_area, &y_ticks, &x_indices, last, lo, hi);
    draw_chop_y_axis(painter, y_axis, plot_area, lo, hi, zoom);
    draw_chop_x_axis(painter, x_axis, plot_area, ns, zoom);

    let lane_h = plot_area.height() / lanes.len() as f32;
    let divider = Color32::from_rgb(44, 46, 52);
    for (li, (ci, ch)) in lanes.iter().enumerate() {
        let lane = Rect::from_min_max(
            Pos2::new(plot_area.min.x, plot_area.min.y + li as f32 * lane_h),
            Pos2::new(plot_area.max.x, plot_area.min.y + (li as f32 + 1.0) * lane_h),
        );
        if li > 0 {
            painter.line_segment(
                [lane.left_top(), lane.right_top()],
                Stroke::new(1.0, divider),
            );
        }
        let label_h = if lane.height() >= 8.0 {
            (lane.height() * 0.42).clamp(10.0, 18.0)
        } else {
            0.0
        };
        let wave_rect = if label_h > 0.0 {
            Rect::from_min_max(
                Pos2::new(lane.min.x, lane.min.y + label_h),
                lane.max,
            )
            .shrink2(Vec2::new(0.0, 1.0))
        } else {
            lane.shrink2(Vec2::new(0.0, 1.0))
        };
        let color = channel_color(*ci);
        let slice = if ch.samples.is_empty() {
            &[0.0][..]
        } else {
            &ch.samples[..ns.min(ch.samples.len())]
        };
        let points = waveform_points(slice, wave_rect, lo, hi);
        let dot_r = (2.2 * zoom).clamp(1.8, 3.2);
        if points.len() >= 2 {
            painter.add(egui::Shape::line(
                points.clone(),
                Stroke::new((1.1 * zoom).clamp(0.8, 1.6), color),
            ));
            for p in [points.first(), points.last()] {
                if let Some(p) = p {
                    painter.circle_filled(*p, dot_r, color);
                }
            }
        } else if points.len() == 1 {
            painter.circle_filled(points[0], dot_r, color);
        }
        if label_h > 0.0 {
            let label_strip = Rect::from_min_max(
                lane.min,
                Pos2::new(lane.max.x, lane.min.y + label_h),
            );
            painter.rect_filled(
                label_strip,
                0.0,
                Color32::from_rgba_unmultiplied(12, 13, 16, 210),
            );
            let font = chop_lane_label_font(zoom);
            let name: String = ch.name.chars().take(16).collect();
            let last_v = slice.last().copied().unwrap_or(0.0);
            paint_chop_text(
                painter,
                label_strip.min + Vec2::new(4.0 * zoom, 1.0),
                Align2::LEFT_TOP,
                name,
                font.clone(),
                Color32::from_rgb(248, 248, 252),
            );
            paint_chop_text(
                painter,
                label_strip.max - Vec2::new(4.0 * zoom, 0.0),
                Align2::RIGHT_TOP,
                format_channel_value(last_v),
                font,
                Color32::from_rgb(255, 220, 120),
            );
        }
    }
}

pub(crate) fn draw_chop_body(app: &OtdApp, ui: &egui::Ui, id: NodeId, body: Rect, zoom: f32) {
    let Some(data) = app.engines.chop_data(id) else {
        return;
    };
    if data.channels.is_empty() {
        return;
    }
    let painter = ui.painter_at(body);
    let ns = displayed_sample_count(data);
    if ns <= 1 {
        draw_chop_single_sample(&painter, &data, body, zoom);
    } else {
        draw_chop_multisample(&painter, &data, body, zoom);
    }
}

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
    let (_, _, plot) = chop_plot_layout(body, zoom);
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
    let snapshot = app.engines.chop_data(id).map(|data| {
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
    });
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
    let cmd_over_channel = cmd
        && pointer.is_some_and(|p| row_rects.iter().any(|row| row.contains(p)));
    for (row_i, (&ci, name)) in channel_indices.iter().zip(names.iter()).enumerate() {
        let row = row_rects[row_i];
        let snippet = channel_expr_snippet(&path, name);
        let payload = ChannelDrag {
            op_path: path.clone(),
            channel: name.clone(),
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
            if matches!(app.drag, Some(crate::canvas::DragState::Node { .. })) {
                app.drag = None;
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
