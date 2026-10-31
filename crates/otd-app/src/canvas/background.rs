use egui::{Color32, Pos2, Rect, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;

use super::bodies::{draw_dat_body, draw_sop_body};
use super::chop::draw_chop_body;
use super::comp_display::background_display_nodes;

pub(crate) fn draw_viewer_backdrops(app: &mut OtdApp, ui: &mut egui::Ui, canvas: Rect) {
    let tiles = mosaic_display_tiles(app, canvas);
    if tiles.is_empty() {
        return;
    }
    let zoom = app.view.zoom;
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
        paint_display_tile(app, ui, id, inner, zoom);
        painter.text(
            tile.left_top() + Vec2::new(8.0, 4.0),
            egui::Align2::LEFT_TOP,
            label,
            egui::FontId::monospace(10.0),
            Color32::from_rgb(200, 202, 212),
        );
    }
}

fn mosaic_display_tiles(app: &OtdApp, canvas: Rect) -> Vec<(NodeId, Rect, String)> {
    if !app.graph.contains(app.current) {
        return Vec::new();
    }
    let ids = background_display_nodes(&app.graph, app.current);
    if ids.is_empty() {
        return Vec::new();
    }
    let n = ids.len();
    let cols = ((n as f32).sqrt().ceil() as usize).max(1);
    let rows = (n + cols - 1) / cols;
    let margin = 10.0;
    let gap = 8.0;
    let avail_w = (canvas.width() - margin * 2.0 - gap * (cols.saturating_sub(1) as f32)).max(1.0);
    let avail_h = (canvas.height() - margin * 2.0 - gap * (rows.saturating_sub(1) as f32)).max(1.0);
    let cell_w = avail_w / cols as f32;
    let cell_h = avail_h / rows as f32;

    ids.into_iter()
        .enumerate()
        .map(|(idx, id)| {
            let col = idx % cols;
            let row = idx / cols;
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

fn paint_display_tile(app: &mut OtdApp, ui: &egui::Ui, id: NodeId, inner: Rect, zoom: f32) {
    match app.graph.node(id).family {
        Family::Top => {
            if let Some((tid, size)) = app.thumbnail(id) {
                let aspect = size[0] as f32 / size[1].max(1) as f32;
                let draw = fit_aspect(inner, aspect);
                ui.painter().with_clip_rect(inner).image(
                    tid,
                    draw,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        Family::Chop => draw_chop_body(app, ui, id, inner, zoom),
        Family::Dat => draw_dat_body(app, ui, id, inner, zoom, Vec2::ZERO),
        Family::Sop => draw_sop_body(app, ui, id, inner, zoom),
        Family::Mat => {
            if let Some(out) = app.graph.resolve_output(id) {
                if app.graph.node(out).family == Family::Top {
                    if let Some((tid, size)) = app.thumbnail(out) {
                        let aspect = size[0] as f32 / size[1].max(1) as f32;
                        let draw = fit_aspect(inner, aspect);
                        ui.painter().with_clip_rect(inner).image(
                            tid,
                            draw,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
        Family::Comp => {}
    }
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
