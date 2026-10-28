use egui::{Pos2, Rect, Vec2};

use super::types::FOOTER_H;

const FLAG_STRIP_W: f32 = 20.0;

pub(crate) struct NodeLayout {
    pub flags: Rect,
    pub body: Rect,
    pub footer: Rect,
}

pub(crate) fn node_layout(rect: Rect, zoom: f32) -> NodeLayout {
    let footer_h = FOOTER_H * zoom;
    let flag_w = FLAG_STRIP_W * zoom;
    let footer = Rect::from_min_max(
        Pos2::new(rect.min.x, rect.max.y - footer_h),
        rect.max,
    );
    let content = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, footer.min.y));
    let flags = Rect::from_min_size(content.min, Vec2::new(flag_w, content.height()));
    let body = Rect::from_min_max(
        Pos2::new(content.min.x + flag_w, content.min.y),
        content.max,
    );
    NodeLayout { flags, body, footer }
}

pub(crate) fn input_port_pos(node_rect: Rect, index: usize, count: usize, zoom: f32) -> Pos2 {
    let layout = node_layout(node_rect, zoom);
    let top = layout.body.min.y;
    let bottom = layout.body.max.y;
    let step = (bottom - top) / (count + 1) as f32;
    Pos2::new(node_rect.min.x, top + step * (index + 1) as f32)
}

pub(crate) fn output_port_pos(node_rect: Rect, index: usize, count: usize, zoom: f32) -> Pos2 {
    let layout = node_layout(node_rect, zoom);
    let top = layout.body.min.y;
    let bottom = layout.body.max.y;
    let step = (bottom - top) / (count.max(1) + 1) as f32;
    Pos2::new(
        node_rect.max.x,
        top + step * (index + 1) as f32,
    )
}

pub(crate) fn port_hit_size(zoom: f32) -> Vec2 {
    Vec2::new(
        (20.0 * zoom).clamp(22.0, 40.0),
        (26.0 * zoom).clamp(24.0, 48.0),
    )
}

pub(crate) fn port_hit_rect(center: Pos2, zoom: f32, input: bool) -> Rect {
    let size = port_hit_size(zoom);
    let mut r = Rect::from_center_size(center, size);
    let pad = (8.0 * zoom).clamp(6.0, 14.0);
    if input {
        r.min.x -= pad;
    } else {
        r.max.x += pad;
    }
    r
}

pub(crate) fn draw_connector_tab(
    painter: &egui::Painter,
    center: Pos2,
    color: egui::Color32,
    zoom: f32,
    input: bool,
) {
    let w = (7.0 * zoom).clamp(6.0, 12.0);
    let h = (15.0 * zoom).clamp(12.0, 22.0);
    let r = if input {
        Rect::from_min_max(
            Pos2::new(center.x - w, center.y - h * 0.5),
            Pos2::new(center.x + w * 0.25, center.y + h * 0.5),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(center.x - w * 0.25, center.y - h * 0.5),
            Pos2::new(center.x + w, center.y + h * 0.5),
        )
    };
    painter.rect_filled(r, 0.0, color);
}
