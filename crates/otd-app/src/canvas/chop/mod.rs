mod drag;
mod paint;
mod plot;
mod tiles;

use egui::Vec2;
use otd_chop::ChopData;
use otd_core::NodeId;

use crate::app::OtdApp;

use super::types::{BODY_H, FOOTER_H, NODE_H};

const CHOP_LANE_H: f32 = BODY_H / 8.0;
const CHOP_MULTISAMPLE_AXIS_H: f32 = 14.0;

pub(crate) use drag::paint_channel_drag_ghost;
pub(crate) use paint::draw_chop_body;
pub(crate) use tiles::chop_tile_channel_handles;

pub(crate) fn chop_node_world_height(data: &ChopData) -> f32 {
    let nc = data.channels.len();
    if nc == 0 {
        return NODE_H;
    }
    let axis = if plot::displayed_sample_count(data) > 1 {
        CHOP_MULTISAMPLE_AXIS_H
    } else {
        0.0
    };
    let body = (nc as f32 * CHOP_LANE_H + axis).max(BODY_H);
    body + FOOTER_H
}

pub(crate) fn node_world_size(app: &OtdApp, id: NodeId) -> Vec2 {
    let main_h = super::callback_embed::main_node_world_height(app, id);
    let extra = super::callback_embed::callback_extra_height(app, id);
    Vec2::new(super::types::NODE_W, main_h + extra)
}

thread_local! {
    static VIEWER_LIMITS: std::cell::Cell<crate::settings::ChopViewerLimits> =
        std::cell::Cell::new(crate::settings::ChopViewerLimits::default());
}

pub(crate) fn set_viewer_limits(limits: crate::settings::ChopViewerLimits) {
    VIEWER_LIMITS.set(limits);
}

pub(crate) fn viewer_limits() -> crate::settings::ChopViewerLimits {
    VIEWER_LIMITS.with(|c| c.get())
}
