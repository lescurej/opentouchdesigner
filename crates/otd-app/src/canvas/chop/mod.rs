mod drag;
mod paint;
mod plot;
mod tiles;

use egui::Vec2;
use otd_core::NodeId;

use crate::app::OtdApp;

pub(crate) use drag::paint_channel_drag_ghost;
pub(crate) use paint::draw_chop_body;
pub(crate) use tiles::chop_tile_channel_handles;

pub(crate) fn node_world_size(app: &OtdApp, id: NodeId) -> Vec2 {
    let main = super::callback_embed::main_node_world_size(app, id);
    let extra = super::callback_embed::callback_extra_height(app, id);
    Vec2::new(main.x, main.y + extra)
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
