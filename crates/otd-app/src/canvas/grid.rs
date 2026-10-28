use egui::{Color32, Pos2, Rect, Stroke};

use super::types::CanvasView;

pub(crate) fn draw_grid(ui: &egui::Ui, rect: Rect, view: CanvasView, origin: Pos2) {
    let painter = ui.painter_at(rect);
    let spacing = 40.0 * view.zoom;
    if spacing < 6.0 {
        return;
    }
    let offset = (view.pan * view.zoom).to_pos2();
    let color = Color32::from_rgb(40, 42, 48);
    let mut x = origin.x + offset.x % spacing;
    while x < rect.max.x {
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            Stroke::new(1.0, color),
        );
        x += spacing;
    }
    let mut y = origin.y + offset.y % spacing;
    while y < rect.max.y {
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            Stroke::new(1.0, color),
        );
        y += spacing;
    }
}
