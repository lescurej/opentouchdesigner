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
pub(crate) mod comp_display;
mod bodies;
mod chop;
mod empty;
mod flags;
mod grid;
pub(crate) mod input;
mod layout;
pub(crate) mod ports;
mod node;
mod palette;
mod types;
mod wires;

pub use palette::channel_color;
pub use types::{CanvasView, DragState, NODE_H, NODE_W};

use egui::{Color32, Pos2, Rect, Sense, Vec2};
use otd_core::NodeId;

use crate::app::OtdApp;

use background::draw_viewer_backdrops;
use empty::empty_hint;
use grid::draw_grid;
use input::{
    draw_marquee, finish_marquee, handle_canvas_scroll, handle_keys, handle_view_input,
    sample_input,
};
use node::draw_node;
use wires::{
    draw_channel_expression_links, draw_pending_wire, draw_wires, finish_wire, handle_wire_click,
};

pub fn show(app: &mut OtdApp, ui: &mut egui::Ui) {
    egui::CentralPanel::no_frame()
        .frame(egui::Frame::NONE.fill(Color32::from_rgb(28, 29, 33)))
        .show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            let response = ui.allocate_rect(rect, Sense::click_and_drag());
            let origin = rect.min;
            app.canvas_rect = rect;

            handle_view_input(app, ui, &response, origin, rect);
            app.input_state = sample_input(ui, rect);
            draw_grid(ui, rect, app.view, origin);

            let nodes: Vec<NodeId> = app.graph.children(app.current).to_vec();
            let mut node_rects: Vec<(NodeId, Rect)> = Vec::new();
            for id in &nodes {
                let p = app.graph.node(*id).pos;
                let min = app.view.to_screen(origin, Vec2::new(p[0], p[1]));
                let size = Vec2::new(NODE_W, NODE_H) * app.view.zoom;
                node_rects.push((*id, Rect::from_min_size(min, size)));
            }

            app.visible = node_rects
                .iter()
                .filter(|(_, r)| rect.intersects(*r))
                .map(|(id, _)| *id)
                .collect();

            draw_viewer_backdrops(app, ui, rect);
            draw_wires(app, ui, origin, &node_rects);
            draw_channel_expression_links(app, ui, &node_rects);
            for (id, node_rect) in &node_rects {
                draw_node(app, ui, *id, *node_rect, origin);
            }
            draw_pending_wire(app, ui, origin, &node_rects);
            draw_marquee(app, ui);
            if nodes.is_empty() {
                empty_hint(app, ui, rect);
            }
            finish_wire(app, ui, &node_rects);
            finish_marquee(app, ui, &node_rects);
            handle_wire_click(app, ui, &node_rects);
            handle_keys(app, ui, origin, rect);
            handle_canvas_scroll(app, ui, origin, rect);
        });
}

pub fn node_at(app: &OtdApp, pos: Pos2) -> Option<NodeId> {
    if !app.canvas_rect.contains(pos) {
        return None;
    }
    let origin = app.canvas_rect.min;
    let size = Vec2::new(NODE_W, NODE_H) * app.view.zoom;
    app.graph
        .children(app.current)
        .iter()
        .rev()
        .copied()
        .find(|id| {
            let p = app.graph.node(*id).pos;
            let min = app.view.to_screen(origin, Vec2::new(p[0], p[1]));
            Rect::from_min_size(min, size).contains(pos)
        })
}
