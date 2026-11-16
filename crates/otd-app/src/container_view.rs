use std::collections::BTreeSet;

use egui::RichText;
use otd_core::{Graph, NodeId, Value};

use crate::app::{OtdApp, draw_panel_comp};
use crate::canvas::comp_display;

pub fn append_cook_roots(graph: &Graph, open: &BTreeSet<NodeId>, roots: &mut Vec<NodeId>) {
    for &id in open {
        if graph.contains(id) {
            comp_display::append_panel_comp_cook_roots(graph, id, roots);
        }
    }
}

pub fn show(app: &mut OtdApp, ctx: &egui::Context) {
    let stale: Vec<NodeId> = app
        .canvas.open_container_views
        .iter()
        .copied()
        .filter(|id| !graph_contains_container(app, *id))
        .collect();
    for id in stale {
        app.canvas.open_container_views.remove(&id);
    }

    let ids: Vec<NodeId> = app.canvas.open_container_views.iter().copied().collect();
    for id in ids {
        let (vw, vh) = otd_engine::panel::container_view_pixel_size(&app.graph, id);
        let path = app.graph.path(id);
        let name = app.graph.node(id).name.clone();
        let title = format!("{name}  ({path})");
        let widgets = otd_engine::panel::panel_display_widgets(&app.graph, id);

        let mut open = true;
        let mut panel_edits = Vec::new();
        egui::Window::new(title)
            .id(egui::Id::new(("container-view", id)))
            .open(&mut open)
            .default_size([vw, vh])
            .min_size([64.0, 64.0])
            .resizable(true)
            .collapsible(true)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("View {vw:.0}×{vh:.0} — edit network in the editor"))
                        .weak()
                        .small(),
                );
                let area = ui.available_rect_before_wrap();
                panel_edits.extend(draw_panel_comp(ui, app, id, area, &widgets, true));
            });

        if !open {
            app.canvas.open_container_views.remove(&id);
        }
        for (wid, value) in panel_edits {
            app.edit("panel");
            let _ = app.graph.set_param(wid, "value", Value::Float(value));
        }
    }
}

fn graph_contains_container(app: &OtdApp, id: NodeId) -> bool {
    app.graph.contains(id) && app.graph.node(id).op_type == otd_engine::panel::CONTAINER
}
