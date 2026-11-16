use egui::{Color32, Rect, Stroke, Vec2};

use crate::params::ChannelDrag;

use super::super::palette::channel_color;
use super::paint::chop_bar_cell;

pub(crate) fn channel_drag_grab_id(op_path: &str, channel: &str) -> egui::Id {
    egui::Id::new(("chan_drag_grab", op_path, channel))
}

fn channel_drag_ghost_size(zoom: f32) -> Vec2 {
    Vec2::new(
        (132.0 * zoom).clamp(108.0, 168.0),
        (20.0 * zoom).clamp(18.0, 26.0),
    )
}

fn draw_channel_drag_ghost(painter: &egui::Painter, rect: Rect, drag: &ChannelDrag, zoom: f32) {
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
