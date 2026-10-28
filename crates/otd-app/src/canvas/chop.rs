use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use otd_chop::ChopData;
use otd_core::NodeId;

use crate::app::OtdApp;
use crate::params::{channel_expr_snippet, copy_channel_expression, ChannelDrag};

use super::palette::channel_color;

const CHOP_TILE_MAX_CHANNELS: usize = 8;
const CHOP_TILE_MAX_SAMPLES: usize = 16;

fn chop_value_bounds(data: &ChopData, channels: usize, samples: usize) -> (f32, f32) {
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    for ch in data.channels.iter().take(channels) {
        for &v in ch.samples.iter().take(samples) {
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

fn chop_autoscale_peak(data: &ChopData, channels: usize, samples: usize) -> f32 {
    let (lo, hi) = chop_value_bounds(data, channels, samples);
    let peak = lo.abs().max(hi.abs());
    if peak < 1e-12 {
        1.0
    } else {
        peak
    }
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
    let font = FontId::proportional((7.5 * zoom).clamp(6.0, 10.0));
    let text = Color32::from_rgb(205, 207, 215);
    let plot = cell;
    let mid_x = plot.center().x;
    painter.line_segment(
        [Pos2::new(mid_x, plot.min.y), Pos2::new(mid_x, plot.max.y)],
        Stroke::new(1.0, Color32::from_rgb(58, 60, 66)),
    );
    let half_w = plot.width() * 0.5;
    let norm = (v / peak).clamp(-1.0, 1.0);
    let x0 = mid_x + norm.min(0.0) * half_w;
    let x1 = mid_x + norm.max(0.0) * half_w;
    painter.rect_filled(
        Rect::from_min_max(
            Pos2::new(x0.min(x1), plot.min.y),
            Pos2::new(x0.max(x1), plot.max.y),
        ),
        0.0,
        bar_color,
    );
    if show_value && zoom > 0.32 {
        painter.text(
            Pos2::new(plot.min.x, plot.center().y),
            Align2::LEFT_CENTER,
            format!("{v:.4}"),
            font.clone(),
            text,
        );
    }
    if let Some(name) = show_name {
        if zoom > 0.32 {
            let short: String = name.chars().take(14).collect();
            painter.text(
                Pos2::new(plot.max.x, plot.center().y),
                Align2::RIGHT_CENTER,
                short,
                font,
                text,
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
    let nc = data.channels.len().min(CHOP_TILE_MAX_CHANNELS);
    if nc == 0 {
        return;
    }
    let peak = chop_autoscale_peak(data, nc, 1);
    let row_h = body.height() / nc as f32;
    let divider = Color32::from_rgb(44, 46, 52);

    for (ci, ch) in data.channels.iter().enumerate().take(nc) {
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

fn draw_chop_sample_grid(
    painter: &egui::Painter,
    data: &ChopData,
    body: Rect,
    zoom: f32,
) {
    let nc = data.channels.len().min(CHOP_TILE_MAX_CHANNELS);
    let ns = data.num_samples().min(CHOP_TILE_MAX_SAMPLES);
    if nc == 0 || ns == 0 {
        return;
    }
    let peak = chop_autoscale_peak(data, nc, ns);
    let band_h = body.height() / ns as f32;
    let divider = Color32::from_rgb(44, 46, 52);

    for si in 0..ns {
        let band = Rect::from_min_max(
            Pos2::new(body.min.x, body.min.y + si as f32 * band_h),
            Pos2::new(body.max.x, body.min.y + (si as f32 + 1.0) * band_h),
        );
        if si > 0 {
            painter.line_segment(
                [band.left_top(), band.right_top()],
                Stroke::new(1.0, divider),
            );
        }
        let cell_w = band.width() / nc as f32;
        for (ci, ch) in data.channels.iter().enumerate().take(nc) {
            let cell = Rect::from_min_max(
                Pos2::new(band.min.x + ci as f32 * cell_w, band.min.y),
                Pos2::new(band.min.x + (ci as f32 + 1.0) * cell_w, band.max.y),
            );
            if ci > 0 {
                painter.line_segment(
                    [cell.left_top(), cell.left_bottom()],
                    Stroke::new(1.0, divider),
                );
            }
            let v = ch.samples.get(si).copied().unwrap_or(0.0);
            let show_name = if si == 0 { Some(ch.name.as_str()) } else { None };
            chop_bar_cell(
                painter,
                cell,
                v,
                peak,
                zoom,
                channel_color(ci),
                true,
                show_name,
            );
        }
    }
}

fn draw_chop_waveforms(
    painter: &egui::Painter,
    data: &ChopData,
    body: Rect,
    zoom: f32,
) {
    let lanes: Vec<_> = data
        .channels
        .iter()
        .enumerate()
        .take(CHOP_TILE_MAX_CHANNELS)
        .filter(|(_, ch)| ch.samples.len() >= 2)
        .collect();
    if lanes.is_empty() {
        return;
    }
    let peak = chop_autoscale_peak(data, lanes.len(), data.num_samples());
    let range = peak * 1.05;
    let lane_h = body.height() / lanes.len() as f32;
    let divider = Color32::from_rgb(44, 46, 52);

    for (li, (ci, ch)) in lanes.iter().enumerate() {
        let lane = Rect::from_min_max(
            Pos2::new(body.min.x, body.min.y + li as f32 * lane_h),
            Pos2::new(body.max.x, body.min.y + (li as f32 + 1.0) * lane_h),
        );
        if li > 0 {
            painter.line_segment(
                [lane.left_top(), lane.right_top()],
                Stroke::new(1.0, divider),
            );
        }
        let plot = lane;
        let mid_y = plot.center().y;
        let half_h = plot.height() * 0.5;
        painter.line_segment(
            [
                Pos2::new(plot.min.x, mid_y),
                Pos2::new(plot.max.x, mid_y),
            ],
            Stroke::new(1.0, Color32::from_rgb(50, 52, 58)),
        );
        let color = channel_color(*ci);
        let n = ch.samples.len();
        let step = plot.width() / (n - 1).max(1) as f32;
        let y_of = |v: f32| mid_y - (v / range).clamp(-1.0, 1.0) * half_h;
        let points: Vec<Pos2> = ch
            .samples
            .iter()
            .enumerate()
            .map(|(i, v)| Pos2::new(plot.min.x + i as f32 * step, y_of(*v)))
            .collect();
        painter.add(egui::Shape::line(
            points,
            Stroke::new((1.1 * zoom).clamp(0.8, 1.6), color),
        ));
        if zoom > 0.42 {
            let name: String = ch.name.chars().take(10).collect();
            painter.text(
                plot.left_top(),
                Align2::LEFT_TOP,
                name,
                FontId::proportional((7.5 * zoom).clamp(6.0, 9.0)),
                color,
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
    let ns = data.num_samples();
    if ns <= 1 {
        draw_chop_single_sample(&painter, &data, body, zoom);
    } else if ns <= CHOP_TILE_MAX_SAMPLES {
        draw_chop_sample_grid(&painter, &data, body, zoom);
    } else {
        draw_chop_waveforms(&painter, &data, body, zoom);
    }
}

pub(crate) fn chop_tile_channel_handles(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: NodeId,
    body: Rect,
    zoom: f32,
) {
    if zoom < 0.32 {
        return;
    }
    let Some(data) = app.engines.chop_data(id) else {
        return;
    };
    if data.num_samples() > 1 {
        return;
    }
    let rows: Vec<String> = data
        .channels
        .iter()
        .take(CHOP_TILE_MAX_CHANNELS)
        .map(|c| c.name.clone())
        .collect();
    if rows.is_empty() {
        return;
    }
    let path = app.graph.path(id);
    let row_h = body.height() / rows.len() as f32;
    let handle_w = (10.0 * zoom).clamp(6.0, 14.0);
    for (i, name) in rows.iter().enumerate() {
        let row = Rect::from_min_max(
            Pos2::new(body.min.x, body.min.y + i as f32 * row_h),
            Pos2::new(body.max.x, body.min.y + (i as f32 + 1.0) * row_h),
        );
        let snippet = channel_expr_snippet(&path, name);
        let payload = ChannelDrag {
            op_path: path.clone(),
            channel: name.clone(),
        };
        let handle = Rect::from_min_size(row.min, Vec2::new(handle_w, row.height()));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(handle));
        let dnd = child.dnd_drag_source(egui::Id::new(("tile_chan", id, i)), payload, |ui| {
            ui.set_min_size(handle.size());
        });
        if dnd.response.double_clicked() {
            copy_channel_expression(app, ui.ctx(), snippet.clone());
        }
        dnd.response.on_hover_text(format!("{snippet}\ndrag from strip · double-click copy"));
    }
}
