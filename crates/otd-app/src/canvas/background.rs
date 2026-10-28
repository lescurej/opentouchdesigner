use egui::{Color32, Pos2, Rect, Vec2};
use otd_core::NodeId;

use crate::app::OtdApp;

use super::comp_display::{background_display_comps, comp_view_top, panel_widgets_local_to_comp};

pub(crate) fn draw_viewer_backdrops(app: &mut OtdApp, ui: &mut egui::Ui, canvas: Rect) {
    let tiles = arrange_background_tiles(app, canvas);
    if tiles.is_empty() {
        return;
    }
    let painter = ui.painter().with_clip_rect(canvas);
    for (id, tile, label) in tiles {
        painter.rect_filled(tile, 4.0, Color32::from_rgb(18, 19, 22));
        painter.rect_stroke(
            tile,
            4.0,
            egui::Stroke::new(1.0, Color32::from_rgb(48, 52, 62)),
            egui::StrokeKind::Inside,
        );
        let inner = tile.shrink(6.0);
        let local = panel_widgets_local_to_comp(&app.graph, id);
        if otd_engine::panel::is_panel_type(&app.graph.node(id).op_type) {
            let local = if local.is_empty() {
                otd_engine::panel::panel_display_widgets(&app.graph, id)
            } else {
                local
            };
            if !local.is_empty() {
                let _ = crate::app::draw_panel_comp(ui, app, id, inner, &local, false);
            }
        } else if let Some(tex_id) = comp_view_top(&app.graph, id) {
            if let Some((tid, size)) = app.thumbnail(tex_id) {
                let aspect = size[0] as f32 / size[1].max(1) as f32;
                let draw = fit_aspect(inner, aspect);
                painter.image(
                    tid,
                    draw,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        painter.text(
            tile.left_top() + Vec2::new(8.0, 4.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::monospace(10.0),
            Color32::from_rgb(200, 202, 212),
        );
    }
}

fn arrange_background_tiles(app: &OtdApp, canvas: Rect) -> Vec<(NodeId, Rect, String)> {
    let ids = background_display_comps(&app.graph, app.current);
    if ids.is_empty() {
        return Vec::new();
    }

    let n = ids.len();
    let cols = ((n as f32).sqrt().ceil() as usize).max(1);
    let rows = (n + cols - 1) / cols;
    let margin = 10.0;
    let gap = 8.0;
    let avail_w = canvas.width() - margin * 2.0 - gap * (cols.saturating_sub(1) as f32);
    let avail_h = canvas.height() - margin * 2.0 - gap * (rows.saturating_sub(1) as f32);
    let cell_w = avail_w / cols as f32;
    let cell_h = avail_h / rows as f32;

    ids.into_iter()
        .enumerate()
        .map(|(i, id)| {
            let col = i % cols;
            let row = i / cols;
            let min = Pos2::new(
                canvas.min.x + margin + col as f32 * (cell_w + gap),
                canvas.min.y + margin + row as f32 * (cell_h + gap),
            );
            let tile = Rect::from_min_size(min, Vec2::new(cell_w, cell_h));
            let label = app.graph.node(id).name.clone();
            (id, tile, label)
        })
        .collect()
}

fn fit_aspect(area: Rect, aspect: f32) -> Rect {
    if aspect > area.width() / area.height().max(1.0) {
        let h = area.width() / aspect;
        Rect::from_center_size(area.center(), Vec2::new(area.width(), h))
    } else {
        let w = area.height() * aspect;
        Rect::from_center_size(area.center(), Vec2::new(w, area.height()))
    }
}
