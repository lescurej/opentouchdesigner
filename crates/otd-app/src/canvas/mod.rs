//! The network editor.
//!
//! PLAN.md §6 flags this as the layer worth owning outright, because it is the
//! product surface. Two things it must get right and does:
//!
//!  * **Every node shows its real output at frame rate.** The node body is the
//!    operator's texture, not a preview render. The network is the debugger.
//!  * **Cook state is legible.** Animated nodes are marked, cached nodes are
//!    dimmed, and each node carries its own cook time.

mod background;
mod bodies;
mod callback_embed;
mod chop;
pub(crate) mod comp_display;
mod viewer_active;
pub(crate) use chop::{node_world_size, paint_channel_drag_ghost};
mod empty;
mod flags;
pub mod grid;
pub(crate) mod input;
mod layout;
mod node;
pub(crate) mod node_message;
mod palette;
mod param_drop;
pub(crate) mod ports;
pub(crate) use param_drop::apply_param_reference;
mod types;
mod wire_curve;
mod wires;

pub(crate) use param_drop::PendingParamDrop;
pub use types::{CanvasView, DragState, NODE_H, NODE_W};

use egui::{Pos2, Rect, Sense, Vec2};
use otd_core::NodeId;

use crate::app::OtdApp;

use background::draw_viewer_backdrops;
use empty::empty_hint;
use grid::draw_grid;
use input::{
    draw_marquee, finish_background_click, finish_marquee, handle_canvas_scroll, handle_keys,
    handle_view_input, sample_input,
};
use node::draw_node;
use param_drop::{draw_param_drop_highlight, draw_param_drop_menu, finish_node_drop};
use wires::{
    draw_callback_dat_links, draw_channel_expression_links, draw_pending_wire, draw_wires,
    finish_wire, handle_wire_click, show_wire_menu,
};

pub fn show(app: &mut OtdApp, ui: &mut egui::Ui) {
    let settings = app.effective_settings().clone();
    let chrome_dark = ui.ctx().theme() == egui::Theme::Dark;
    let mut theme = crate::ui_palette::network_for_appearance(settings.ui_palette, chrome_dark);
    crate::ui_palette::apply_network_appearance(&mut theme, &settings);
    palette::set_active(theme);
    chop::set_viewer_limits(crate::settings::ChopViewerLimits::from_settings(&settings));
    let grid_draw = crate::settings::NetworkGridDraw::from_settings(&settings);
    egui::CentralPanel::no_frame()
        .frame(egui::Frame::NONE.fill(palette::canvas_bg()))
        .show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            ui.set_clip_rect(rect);
            let response = ui.allocate_rect(rect, Sense::click_and_drag());
            app.canvas.network_view_id = Some(response.id);
            let origin = rect.min;
            app.canvas.canvas_rect = rect;

            handle_view_input(app, ui, &response, rect);
            app.input_state = sample_input(ui, rect);
            draw_viewer_backdrops(app, ui, rect);
            draw_grid(
                ui,
                rect,
                app.canvas.view,
                origin,
                palette::grid_color(),
                palette::grid_dots(),
                grid_draw,
            );

            let nodes: Vec<NodeId> = if app.graph.contains(app.canvas.current) {
                app.graph.children(app.canvas.current).to_vec()
            } else {
                Vec::new()
            };
            let mut node_rects: Vec<(NodeId, Rect)> = Vec::new();
            for id in &nodes {
                if hidden_on_canvas(app, *id) {
                    continue;
                }
                let p = app.graph.node(*id).pos;
                let min = app.canvas.view.to_screen(origin, Vec2::new(p[0], p[1]));
                let size = node_world_size(app, *id) * app.canvas.view.zoom;
                node_rects.push((*id, Rect::from_min_size(min, size)));
            }

            app.canvas.visible = node_rects
                .iter()
                .filter(|(_, r)| rect.intersects(*r))
                .map(|(id, _)| *id)
                .collect();

            let on_screen: Vec<(NodeId, Rect)> = node_rects
                .iter()
                .filter(|(_, r)| rect.intersects(*r))
                .copied()
                .collect();

            draw_wires(app, ui, &response, origin, rect, &node_rects);
            viewer_active::prune_viewer_state(app);
            draw_callback_dat_links(app, ui, rect, &node_rects);
            draw_channel_expression_links(app, ui, rect, &node_rects);
            for (id, node_rect) in &on_screen {
                draw_node(app, ui, *id, *node_rect, origin);
            }
            draw_pending_wire(app, ui, origin, &node_rects);
            draw_param_drop_highlight(app, ui, &node_rects);
            draw_marquee(app, ui);
            if nodes.is_empty() {
                empty_hint(app, ui, rect);
            }
            finish_node_drop(app, ui, &node_rects);
            finish_wire(app, ui, &node_rects);
            finish_marquee(app, ui, &node_rects);
            handle_wire_click(app, ui, &response, &node_rects);
            finish_background_click(app, ui, &response, origin);
            show_wire_menu(app, ui);
            handle_keys(app, ui, origin, rect);
            draw_param_drop_menu(app, ui.ctx());
            paint_channel_drag_ghost(ui.ctx(), app.canvas.view.zoom);
            handle_canvas_scroll(app, ui, origin, rect);
        });
}

fn hidden_on_canvas(app: &OtdApp, id: NodeId) -> bool {
    !app.graph.node(id).dock_shown || callback_embed::hide_callback_dat_on_canvas(app, id)
}

pub fn node_at(app: &OtdApp, pos: Pos2) -> Option<NodeId> {
    if !app.canvas.canvas_rect.contains(pos) {
        return None;
    }
    let origin = app.canvas.canvas_rect.min;
    app.graph
        .children(app.canvas.current)
        .iter()
        .rev()
        .copied()
        .filter(|id| !hidden_on_canvas(app, *id))
        .find(|id| {
            let p = app.graph.node(*id).pos;
            let min = app.canvas.view.to_screen(origin, Vec2::new(p[0], p[1]));
            let size = node_world_size(app, *id) * app.canvas.view.zoom;
            Rect::from_min_size(min, size).contains(pos)
        })
}
