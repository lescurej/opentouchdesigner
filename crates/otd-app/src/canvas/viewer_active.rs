use egui::{CursorIcon, Rect, Sense, Ui, Vec2};
use otd_core::NodeId;

use crate::app::OtdApp;

pub(crate) fn viewer_operating(
    ui: &Ui,
    viewer_on: bool,
    viewer_active: bool,
) -> bool {
    if !viewer_on {
        return false;
    }
    viewer_active
        || ui.input(|i| i.modifiers.alt && !i.modifiers.command && !i.modifiers.ctrl)
}

pub(crate) fn top_viewer_screen_rect(body: Rect, aspect: f32, pan: Vec2, zoom: f32) -> Rect {
    let fit = if aspect > body.width() / body.height() {
        let h = body.width() / aspect;
        Rect::from_center_size(body.center(), Vec2::new(body.width(), h))
    } else {
        let w = body.height() * aspect;
        Rect::from_center_size(body.center(), Vec2::new(w, body.height()))
    };
    let z = zoom.clamp(0.25, 8.0);
    Rect::from_center_size(fit.center() + pan, fit.size() * z)
}

pub(crate) fn handle_top_viewer_input(
    app: &mut OtdApp,
    ui: &mut Ui,
    id: NodeId,
    body: Rect,
    aspect: f32,
) {
    let resp = ui.interact(
        body,
        egui::Id::new(("viewer_top", id)),
        Sense::click_and_drag(),
    );
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    if resp.dragged() {
        let entry = app.viewer_top_pan.entry(id).or_default();
        *entry += resp.drag_delta();
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.0 {
            let z = app.viewer_top_zoom.entry(id).or_insert(1.0);
            *z *= (1.0 - scroll * 0.002).clamp(0.85, 1.15);
            *z = z.clamp(0.25, 8.0);
        }
    }
    let _ = aspect;
}

pub(crate) fn top_viewer_screen_rect_for(app: &OtdApp, id: NodeId, body: Rect, aspect: f32) -> Rect {
    let pan = app.viewer_top_pan.get(&id).copied().unwrap_or_default();
    let zoom = app.viewer_top_zoom.get(&id).copied().unwrap_or(1.0);
    top_viewer_screen_rect(body, aspect, pan, zoom)
}

pub(crate) fn apply_panel_edits(app: &mut OtdApp, edits: &[(NodeId, f64)]) {
    if edits.is_empty() {
        return;
    }
    app.edit("panel");
    for (id, value) in edits {
        let _ = app
            .graph
            .set_param(*id, "value", otd_core::Value::Float(*value));
    }
    app.history.end_gesture();
}

pub(crate) fn dat_viewer_scroll(app: &OtdApp, id: NodeId) -> Vec2 {
    app.viewer_dat_scroll.get(&id).copied().unwrap_or_default()
}

pub(crate) fn handle_dat_viewer_input(app: &mut OtdApp, ui: &mut Ui, id: NodeId, body: Rect) {
    let resp = ui.interact(
        body,
        egui::Id::new(("viewer_dat", id)),
        Sense::click_and_drag(),
    );
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }
    let scroll = app.viewer_dat_scroll.entry(id).or_default();
    if resp.dragged() {
        *scroll -= resp.drag_delta();
        scroll.x = scroll.x.max(0.0);
        scroll.y = scroll.y.max(0.0);
    }
    if resp.hovered() {
        let wheel = ui.input(|i| i.smooth_scroll_delta);
        if wheel != Vec2::ZERO {
            scroll.y -= wheel.y;
            scroll.x -= wheel.x;
            scroll.x = scroll.x.max(0.0);
            scroll.y = scroll.y.max(0.0);
        }
    }
}

pub(crate) fn prune_viewer_state(app: &mut OtdApp) {
    app.viewer_top_pan
        .retain(|id, _| app.graph.contains(*id));
    app.viewer_top_zoom
        .retain(|id, _| app.graph.contains(*id));
    app.viewer_dat_scroll
        .retain(|id, _| app.graph.contains(*id));
}