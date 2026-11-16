use egui::{Color32, Pos2, Rect, Stroke, StrokeKind};
use otd_core::NodeId;

use crate::app::OtdApp;

use crate::canvas::chop::node_world_size;
use crate::canvas::types::{CanvasView, DragState};

pub(crate) fn frame_all_nodes(app: &mut OtdApp) {
    let canvas = app.canvas.canvas_rect;
    if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
        return;
    }
    let origin = canvas.min;
    if !app.graph.contains(app.canvas.current) {
        return;
    }
    let ids: Vec<NodeId> = app.graph.children(app.canvas.current).to_vec();
    let next = if ids.is_empty() {
        CanvasView::default()
    } else {
        let mut min = egui::Vec2::new(f32::INFINITY, f32::INFINITY);
        let mut max = egui::Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for id in ids {
            let p = app.graph.node(id).pos;
            min.x = min.x.min(p[0]);
            min.y = min.y.min(p[1]);
            let size = node_world_size(app, id);
            max.x = max.x.max(p[0] + size.x);
            max.y = max.y.max(p[1] + size.y);
        }
        CanvasView::fit_world_bounds(origin, canvas, min, max)
    };
    if next.pan != app.canvas.view.pan || (next.zoom - app.canvas.view.zoom).abs() > 1e-4 {
        app.canvas.view = next;
        app.mark_view_dirty();
    }
}

pub(crate) fn handle_view_input(
    app: &mut OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    _rect: Rect,
) {
    let shift = ui.input(|i| i.modifiers.shift);
    if response.drag_started() && shift {
        if let Some(p) = ui.ctx().pointer_interact_pos() {
            app.canvas.drag = Some(DragState::Marquee {
                start: p,
                additive: true,
            });
        }
    }
    if response.clicked() || response.drag_started() || response.dragged() {
        response.request_focus();
    }
    if response.dragged() && app.canvas.drag.is_none() {
        let delta = response.drag_delta() / app.canvas.view.zoom;
        if delta.x != 0.0 || delta.y != 0.0 {
            app.canvas.view.pan += delta;
            app.mark_view_dirty();
        }
    }
}

pub(crate) fn finish_background_click(
    app: &mut OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    origin: Pos2,
) {
    if response.clicked() && app.canvas.drag.is_none() {
        if app.canvas.skip_canvas_deselect_once {
            app.canvas.skip_canvas_deselect_once = false;
        } else {
            release_param_focus(ui.ctx());
            app.clear_selection();
        }
    }
    if response.double_clicked() && app.canvas.drag.is_none() {
        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            ui.ctx()
                .memory_mut(|mem| mem.move_focus(egui::FocusDirection::None));
            let world_pos = app.canvas.view.to_world(origin, pointer);
            app.open_create_operator_dialog(world_pos);
        }
    }
}

pub(crate) fn draw_marquee(app: &OtdApp, ui: &egui::Ui) {
    let Some(DragState::Marquee { start, .. }) = app.canvas.drag else {
        return;
    };
    let Some(p) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let r = Rect::from_two_pos(start, p);
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Color32::from_rgb(120, 200, 130)),
        StrokeKind::Outside,
    );
    ui.painter()
        .rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(100, 200, 120, 28));
}

pub(crate) fn finish_marquee(app: &mut OtdApp, ui: &egui::Ui, node_rects: &[(NodeId, Rect)]) {
    if !ui.ctx().input(|i| i.pointer.any_released()) {
        return;
    }
    let Some(DragState::Marquee { start, additive }) = app.canvas.drag.take() else {
        return;
    };
    let Some(p) = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.ctx().pointer_latest_pos())
    else {
        return;
    };
    let sel = Rect::from_two_pos(start, p);
    if sel.width() < 3.0 && sel.height() < 3.0 {
        return;
    }
    let hits: Vec<NodeId> = node_rects
        .iter()
        .filter(|(_, rect)| sel.intersects(*rect))
        .map(|(id, _)| *id)
        .collect();
    if hits.is_empty() {
        return;
    }
    release_param_focus(ui.ctx());
    app.canvas.selected_wire = None;
    app.canvas.skip_canvas_deselect_once = true;
    if additive {
        for id in &hits {
            app.canvas.selection.insert(*id);
        }
        if let Some(last) = hits.last() {
            app.canvas.selected = Some(*last);
        }
    } else {
        app.canvas.selection.clear();
        for id in &hits {
            app.canvas.selection.insert(*id);
        }
        app.canvas.selected = hits.first().copied();
    }
}

pub(crate) fn handle_canvas_scroll(app: &mut OtdApp, ui: &egui::Ui, origin: Pos2, rect: Rect) {
    let Some(pointer) = ui.ctx().pointer_hover_pos() else {
        return;
    };
    if !crate::shortcut_routing::canvas_accepts_wheel_zoom(app, ui.ctx()) {
        return;
    }
    if !rect.contains(pointer) {
        return;
    }
    let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() > 0.0 {
        let before = app.canvas.view.to_world(origin, pointer);
        let zoom = (app.canvas.view.zoom * (1.0 + scroll * 0.002)).clamp(0.15, 3.0);
        if (zoom - app.canvas.view.zoom).abs() > f32::EPSILON {
            app.canvas.view.zoom = zoom;
            let after = app.canvas.view.to_world(origin, pointer);
            app.canvas.view.pan += after - before;
            app.mark_view_dirty();
        }
    }
}

pub(crate) fn release_param_focus(ctx: &egui::Context) {
    ctx.memory_mut(|mem| mem.move_focus(egui::FocusDirection::None));
}

pub(crate) fn label_text_selected(ctx: &egui::Context) -> bool {
    ctx.plugin::<egui::text_selection::LabelSelectionState>()
        .lock()
        .has_selection()
}

pub(crate) fn sample_input(ui: &egui::Ui, rect: Rect) -> otd_chop::InputState {
    let ctx = ui.ctx();
    let pos = ctx.pointer_latest_pos().unwrap_or(rect.center());
    ctx.input(|i| otd_chop::InputState {
        mouse: [
            (pos.x - rect.min.x) / rect.width().max(1.0) - 0.5,
            0.5 - (pos.y - rect.min.y) / rect.height().max(1.0),
        ],
        buttons: [
            i.pointer.primary_down(),
            i.pointer.secondary_down(),
            i.pointer.middle_down(),
        ],
        wheel: i.smooth_scroll_delta.y,
        keys: i
            .keys_down
            .iter()
            .map(|k| format!("{k:?}").to_lowercase())
            .collect(),
    })
}
