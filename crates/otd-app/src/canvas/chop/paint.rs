use egui::{Align2, Color32, Pos2, Rect, Stroke, Vec2};
use otd_chop::ChopData;
use otd_core::NodeId;

use crate::app::OtdApp;

use super::super::palette::channel_color;
use super::plot::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn chop_bar_cell(
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

fn draw_chop_single_sample(painter: &egui::Painter, data: &ChopData, body: Rect, zoom: f32) {
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
            painter.line_segment([row.left_top(), row.right_top()], Stroke::new(1.0, divider));
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

fn waveform_points(samples: &[f32], plot: Rect, lo: f32, hi: f32) -> Vec<Pos2> {
    let n = samples.len();
    if n == 0 {
        return Vec::new();
    }
    let span = (hi - lo).max(1e-12);
    let y_of = |v: f32| plot.max.y - ((v - lo) / span).clamp(0.0, 1.0) * plot.height();
    if n == 1 {
        return vec![Pos2::new(plot.center().x, y_of(samples[0]))];
    }
    let px = (plot.width().ceil().max(2.0) as usize).min(super::viewer_limits().max_waveform_px.max(2));
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

fn draw_chop_multisample(painter: &egui::Painter, data: &ChopData, body: Rect, zoom: f32) {
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
    let max_x_ticks =
        ((plot_area.width() / (36.0 * zoom).clamp(28.0, 44.0)).floor() as usize).clamp(3, 8);
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
            Pos2::new(
                plot_area.max.x,
                plot_area.min.y + (li as f32 + 1.0) * lane_h,
            ),
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
            Rect::from_min_max(Pos2::new(lane.min.x, lane.min.y + label_h), lane.max)
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
            for p in [points.first(), points.last()].into_iter().flatten() {
                painter.circle_filled(*p, dot_r, color);
            }
        } else if points.len() == 1 {
            painter.circle_filled(points[0], dot_r, color);
        }
        if label_h > 0.0 {
            let label_strip =
                Rect::from_min_max(lane.min, Pos2::new(lane.max.x, lane.min.y + label_h));
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
    let Some(data) = app.snapshot_chop(id) else {
        return;
    };
    if data.channels.is_empty() {
        return;
    }
    let painter = ui.painter_at(body);
    let ns = displayed_sample_count(data);
    if ns <= 1 {
        draw_chop_single_sample(&painter, data, body, zoom);
    } else {
        draw_chop_multisample(&painter, data, body, zoom);
    }
}
