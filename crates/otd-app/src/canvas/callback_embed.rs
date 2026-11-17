use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;
use crate::canvas::input::release_param_focus;

use super::types::{CALLBACK_FOLD_H, MIN_NODE_H, MIN_NODE_W, NODE_H, NODE_W};

pub(crate) fn owner_callback_dat(app: &OtdApp, owner: NodeId) -> Option<NodeId> {
    app.graph
        .owned_callback_dat(owner)
        .or_else(|| otd_dat::callbacks::resolved_callbacks_dat(&app.graph, owner))
}

pub(crate) fn node_has_callback(app: &OtdApp, owner: NodeId) -> bool {
    owner_callback_dat(app, owner).is_some()
}

pub(crate) fn callback_embedded(app: &OtdApp, owner: NodeId) -> bool {
    app.canvas.callback_embedded.contains(&owner)
}

pub(crate) fn hide_callback_dat_on_canvas(app: &OtdApp, id: NodeId) -> bool {
    let owner = app.graph.node(id).callback_owner.or_else(|| {
        app.graph
            .callback_dat_links()
            .find_map(|(owner, dat)| (dat == id).then_some(owner))
    });
    let Some(owner) = owner else {
        return false;
    };
    callback_embedded(app, owner)
}

pub(crate) fn callback_extra_height(app: &OtdApp, owner: NodeId) -> f32 {
    if node_has_callback(app, owner) {
        CALLBACK_FOLD_H
    } else {
        0.0
    }
}

pub(crate) fn main_node_world_size(app: &OtdApp, id: NodeId) -> Vec2 {
    match app.graph.node(id).size {
        Some([w, h]) => Vec2::new(w.max(MIN_NODE_W), h.max(MIN_NODE_H)),
        None => Vec2::new(NODE_W, NODE_H),
    }
}

pub(crate) fn main_node_world_height(app: &OtdApp, id: NodeId) -> f32 {
    main_node_world_size(app, id).y
}

pub(crate) fn fold_strip_rect(full_rect: Rect, main_h_world: f32, zoom: f32) -> Rect {
    let main_h_screen = main_h_world * zoom;
    let h = (CALLBACK_FOLD_H * zoom).clamp(13.0, 20.0);
    Rect::from_min_max(
        Pos2::new(full_rect.min.x, full_rect.min.y + main_h_screen),
        Pos2::new(full_rect.max.x, full_rect.min.y + main_h_screen + h),
    )
}

pub(crate) fn draw_fold_control(
    app: &mut OtdApp,
    ui: &mut Ui,
    owner: NodeId,
    strip: Rect,
    zoom: f32,
) -> bool {
    let collapsed = callback_embedded(app, owner);
    let painter = ui.painter();
    let accent = super::palette::family_color(Family::Dat);
    let fill = if collapsed {
        Color32::from_rgb(38, 34, 48)
    } else {
        Color32::from_rgb(58, 48, 72)
    };
    painter.rect_filled(strip, 0.0, fill);
    painter.rect_stroke(
        strip,
        0.0,
        Stroke::new((1.25 * zoom).clamp(1.0, 1.5), accent.gamma_multiply(0.85)),
        StrokeKind::Inside,
    );
    let chevron = if collapsed { "▸" } else { "▾" };
    let label = format!("{chevron}  Callbacks");
    let text_c = if collapsed {
        Color32::from_rgb(175, 165, 195)
    } else {
        Color32::from_rgb(235, 228, 248)
    };
    painter.text(
        strip.left_center() + Vec2::new(5.0 * zoom, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional((9.5 * zoom).clamp(8.0, 12.0)),
        text_c,
    );
    let resp = ui.interact(
        strip,
        egui::Id::new(("callback_fold", owner)),
        Sense::click(),
    );
    if resp.clicked() {
        release_param_focus(ui.ctx());
        if collapsed {
            app.canvas.callback_embedded.remove(&owner);
        } else {
            app.canvas.callback_embedded.insert(owner);
        }
        return true;
    }
    resp.on_hover_text(if collapsed {
        "Show callbacks DAT on the network"
    } else {
        "Fold — hide callbacks DAT from the network"
    });
    false
}
