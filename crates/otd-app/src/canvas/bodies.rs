use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use otd_core::NodeId;

use crate::app::OtdApp;

pub(crate) fn draw_sop_body(app: &OtdApp, ui: &egui::Ui, id: NodeId, body: Rect, zoom: f32) {
    let Some(geo) = app.engines.geometry_of(id) else {
        return;
    };
    if geo.points.is_empty() {
        return;
    }
    let painter = ui.painter_at(body);
    let (bmin, bmax) = geo.bounds();
    let sx = (bmax[0] - bmin[0]).max(1e-4);
    let sy = (bmax[1] - bmin[1]).max(1e-4);
    let pad = 4.0;
    let w = body.width() - pad * 2.0;
    let h = body.height() - pad * 2.0;
    let to_screen = |p: [f32; 3]| {
        Pos2::new(
            body.min.x + pad + (p[0] - bmin[0]) / sx * w,
            body.min.y + pad + (1.0 - (p[1] - bmin[1]) / sy) * h,
        )
    };
    let color = Color32::from_rgb(120, 180, 210);
    for pt in geo.points.iter().take(1200) {
        painter.circle_filled(to_screen(pt.position), 1.1 * zoom.max(0.4), color);
    }
    if !geo.indices.is_empty() {
        let stroke = Stroke::new(1.0 * zoom.max(0.4), color);
        for pair in geo.indices.chunks(2).take(400) {
            if pair.len() == 2 {
                let a = geo.points.get(pair[0] as usize);
                let b = geo.points.get(pair[1] as usize);
                if let (Some(a), Some(b)) = (a, b) {
                    painter.line_segment([to_screen(a.position), to_screen(b.position)], stroke);
                }
            }
        }
    }
    if zoom > 0.45 {
        painter.text(
            body.left_bottom() + Vec2::new(3.0, -2.0),
            Align2::LEFT_BOTTOM,
            format!("{} pts", geo.num_points()),
            FontId::proportional(8.0 * zoom),
            color,
        );
    }
}

pub(crate) fn draw_dat_body(app: &OtdApp, ui: &egui::Ui, id: NodeId, body: Rect, zoom: f32) {
    let Some(data) = app.engines.dat_data(id) else {
        return;
    };
    if zoom < 0.45 || data.rows.is_empty() {
        return;
    }
    let painter = ui.painter_at(body);

    if data.is_text {
        let line = 10.0 * zoom;
        let font = FontId::monospace(8.5 * zoom);
        let text = data.as_text();
        let max_lines = ((body.height() / line) as usize).max(1);
        for (i, line_text) in text.lines().take(max_lines).enumerate() {
            let shown: String = line_text.chars().take(40).collect();
            painter.text(
                body.left_top() + Vec2::new(4.0, 2.0 + i as f32 * line),
                Align2::LEFT_TOP,
                shown,
                font.clone(),
                Color32::from_rgb(160, 162, 172),
            );
        }
        return;
    }

    let cols = data.num_cols().max(1);
    let col_w = body.width() / cols as f32;
    let line = 11.0 * zoom;
    let font = FontId::monospace(8.5 * zoom);
    let max_rows = ((body.height() / line) as usize).max(1);

    for c in 0..=cols {
        let x = body.min.x + c as f32 * col_w;
        painter.line_segment(
            [Pos2::new(x, body.min.y), Pos2::new(x, body.max.y)],
            Stroke::new(1.0, Color32::from_rgb(42, 44, 50)),
        );
    }

    for (r, row) in data.rows.iter().take(max_rows).enumerate() {
        let y = body.min.y + r as f32 * line;
        painter.line_segment(
            [Pos2::new(body.min.x, y), Pos2::new(body.max.x, y)],
            Stroke::new(1.0, Color32::from_rgb(42, 44, 50)),
        );
        let colour = if r == 0 {
            Color32::from_rgb(200, 202, 212)
        } else {
            Color32::from_rgb(150, 152, 162)
        };
        for (c, cell) in row.iter().enumerate().take(cols) {
            let shown: String = cell.chars().take(12).collect();
            painter.text(
                Pos2::new(body.min.x + c as f32 * col_w + 3.0, y + 1.0),
                Align2::LEFT_TOP,
                shown,
                font.clone(),
                colour,
            );
        }
    }
    if data.num_rows() > max_rows {
        painter.text(
            body.right_bottom() - Vec2::new(4.0, 2.0),
            Align2::RIGHT_BOTTOM,
            format!("+{} rows", data.num_rows() - max_rows),
            FontId::proportional(8.5 * zoom),
            Color32::from_rgb(110, 112, 122),
        );
    }
}
