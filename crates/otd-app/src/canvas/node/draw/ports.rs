use egui::{Rect, Sense};
use otd_core::NodeId;

use crate::app::OtdApp;
use crate::canvas::layout::{draw_connector_tab, input_port_pos, output_port_pos, port_hit_rect};
use crate::canvas::palette::family_color;
use crate::canvas::ports::{output_port_count, output_port_family};
use crate::canvas::types::DragState;

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_ports(
    app: &OtdApp,
    painter: &egui::Painter,
    id: NodeId,
    main_rect: Rect,
    zoom: f32,
    input_count: usize,
    input_slots: &[Option<NodeId>],
    input_families: &[otd_core::Family],
    family: otd_core::Family,
) {
    let out_count = output_port_count(&app.graph, id);
    for o in 0..out_count {
        let out_pos = output_port_pos(main_rect, o, out_count, zoom);
        let oc = output_port_family(&app.graph, id, o);
        draw_connector_tab(painter, out_pos, family_color(oc), zoom, false);
    }
    for (i, slot) in input_slots.iter().enumerate() {
        let p = input_port_pos(main_rect, i, input_count, zoom);
        let filled = slot.is_some();
        let accepts = input_families.get(i).copied().unwrap_or(family);
        let port_color = family_color(accepts);
        let color = if filled {
            port_color
        } else {
            port_color.gamma_multiply(0.55)
        };
        draw_connector_tab(painter, p, color, zoom, true);
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn wire_port_interaction(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: NodeId,
    main_rect: Rect,
    zoom: f32,
    locked: bool,
    input_count: usize,
    input_slots: &[Option<NodeId>],
) {
    let out_count = output_port_count(&app.graph, id);
    for o in 0..out_count {
        let out_pos = output_port_pos(main_rect, o, out_count, zoom);
        let out_rect = port_hit_rect(out_pos, zoom, false);
        let out_resp = ui.interact(
            out_rect,
            egui::Id::new(("out", id, o)),
            Sense::click_and_drag(),
        );
        if out_resp.drag_started() && !locked {
            app.canvas.drag = Some(DragState::Wire {
                from: id,
                output: o,
            });
        }
    }
    for i in 0..input_count {
        let wired = input_slots.get(i).and_then(|s| *s).is_some();
        let p = input_port_pos(main_rect, i, input_count, zoom);
        let r = port_hit_rect(p, zoom, true);
        let resp = ui.interact(r, egui::Id::new(("in", id, i)), Sense::click_and_drag());
        if resp.drag_started() && !locked {
            app.canvas.drag = Some(DragState::WireTo {
                dst: id,
                input: i,
                had_wire: wired,
            });
        }
        if resp.secondary_clicked() && wired && !locked {
            app.queue_disconnect(id, i);
            app.canvas.selected_wire = None;
        }
    }
}