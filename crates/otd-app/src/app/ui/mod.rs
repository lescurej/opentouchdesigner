//! Main-window frame loop, chrome, perform/output viewports, and panel drawing.

use super::*;

pub(crate) mod cook_roots;
mod frame;
mod monitor;
mod panel_draw;
mod perform;
mod side_panel;
mod status_bar;
mod timeline;
mod top_bar;
mod viewports;

pub(crate) use panel_draw::{draw_panel_comp, draw_texture_stretch};
pub(super) use panel_draw::{draw_panel, draw_panel_texture, letterbox};