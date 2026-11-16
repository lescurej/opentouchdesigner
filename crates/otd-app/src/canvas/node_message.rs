use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind};
use otd_engine::NodeMessageLevel;

use crate::app::OtdApp;
use crate::canvas::types::FOOTER_H;

pub fn stroke_color(level: NodeMessageLevel) -> Color32 {
    match level {
        NodeMessageLevel::Error => Color32::from_rgb(220, 90, 90),
        NodeMessageLevel::Warning => Color32::from_rgb(230, 165, 75),
    }
}

pub fn text_color(level: NodeMessageLevel) -> Color32 {
    match level {
        NodeMessageLevel::Error => Color32::from_rgb(255, 145, 145),
        NodeMessageLevel::Warning => Color32::from_rgb(255, 200, 120),
    }
}

pub fn paint_node_message(
    app: &OtdApp,
    painter: &egui::Painter,
    id: otd_core::NodeId,
    rect: Rect,
    zoom: f32,
) {
    let Some(msg) = app.snapshot_node_message(id) else {
        return;
    };
    let color = stroke_color(msg.level);
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new((2.0 * zoom).clamp(1.5, 3.0), color),
        StrokeKind::Outside,
    );
    if zoom <= 0.35 {
        return;
    }
    let label = truncate_status(&msg.text, 48);
    let y = rect.center_bottom().y + FOOTER_H * zoom + (6.0 * zoom).max(3.0);
    painter.text(
        Pos2::new(rect.center().x, y),
        Align2::CENTER_TOP,
        label,
        FontId::proportional((9.5 * zoom).clamp(8.0, 12.0)),
        text_color(msg.level),
    );
}

fn truncate_status(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>()
        + "…"
}
