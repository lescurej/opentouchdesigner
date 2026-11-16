use egui::Rect;
use otd_core::{Family, NodeId};

use crate::app::OtdApp;
use crate::canvas::flags::display_flag_allowed;
use crate::canvas::input::{SelectionFlag, toggle_flag_at};

pub(crate) fn node_menu(app: &mut OtdApp, resp: &egui::Response, id: NodeId, _name_bar: Rect) {
    resp.context_menu(|ui| {
        if !app.is_selected(id) {
            app.select_only(id);
        }

        let viewer_on = app.graph.node(id).flags.viewer;
        if ui
            .selectable_label(viewer_on, "Viewer")
            .on_hover_text("Show the operator viewer on this node")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Viewer, id);
            ui.close();
        }
        if display_flag_allowed(app.graph.node(id).family) {
            let bg = app.graph.node(id).flags.bg_display;
            if ui
                .selectable_label(bg, "Display")
                .on_hover_text("Include this operator in the network background mosaic (D)")
                .clicked()
            {
                toggle_flag_at(app, SelectionFlag::Display, id);
                ui.close();
            }
        }
        if app.graph.node(id).family == Family::Comp && ui.button("Enter").clicked() {
            app.queue_enter_network(id);
            ui.close();
        }
        if otd_engine::panel::is_panel_type(&app.graph.node(id).op_type)
            && ui.button("View…").on_hover_text(
                "containerCOMP: floating View window (View page size). Other panels: Perform preview.",
            )
            .clicked()
        {
            app.view_panel_comp(id);
            ui.close();
        }
        if app.graph.node(id).family == Family::Comp
            && ui.button("Set Perform UI").clicked()
        {
            app.set_perform_ui(id);
            ui.close();
        }
        if app.graph.node(id).family == Family::Dat
            && ui
                .button("Open in External Editor")
                .on_hover_text("Write the DAT to a scratch file and open it")
                .clicked()
        {
            if let Err(e) = crate::dat_external::open_in_external_editor(app, id) {
                app.document.status = e;
            }
            ui.close();
        }
        if app.graph.node(id).param("file").is_some() && ui.button("Replace File…").clicked() {
            crate::media::replace_file(app, id);
            ui.close();
        }

        ui.separator();
        if ui.button("Copy").on_hover_text("Cmd+C").clicked() {
            app.copy_selection(ui.ctx());
            ui.close();
        }
        if ui.button("Cut").on_hover_text("Cmd+X").clicked() {
            app.cut_selection(ui.ctx());
            ui.close();
        }
        if ui.button("Paste").on_hover_text("Cmd+V").clicked() {
            let pointer = ui.ctx().pointer_latest_pos().unwrap_or(resp.rect.center());
            let origin = app.canvas.canvas_rect.min;
            let world = app.canvas.view.to_world(origin, pointer);
            app.queue_paste_clipboard([world.x, world.y]);
            ui.close();
        }

        ui.separator();
        let family = app.graph.node(id).family;
        ui.menu_button(format!("Add {}", family.suffix()), |ui| {
            crate::media::add_operator_menu(app, ui, Some(family));
        });

        let wired: Vec<(usize, otd_core::NodeId)> = app
            .graph
            .node(id)
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|src| (i, src)))
            .collect();
        if !wired.is_empty() && !app.graph.node(id).flags.lock {
            ui.menu_button("Disconnect", |ui| {
                for (i, src) in &wired {
                    let label = format!(
                        "Input {} ← {}",
                        i,
                        app.graph.path(*src).trim_start_matches('/')
                    );
                    if ui.button(label).clicked() {
                        app.queue_disconnect(id, *i);
                        app.canvas.selected_wire = None;
                        ui.close();
                    }
                }
            });
            ui.separator();
        }
        let bypassed = app.graph.node(id).flags.bypass;
        if ui
            .selectable_label(bypassed, "Bypass")
            .on_hover_text("Pass the input straight through — B")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Bypass, id);
            ui.close();
        }
        let locked = app.graph.node(id).flags.lock;
        if ui
            .selectable_label(locked, "Lock")
            .on_hover_text("Block edits and wiring — L")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Lock, id);
            ui.close();
        }
        let immune = app.graph.node(id).flags.clone_immune;
        if ui
            .selectable_label(immune, "Clone Immune")
            .on_hover_text("Keep local changes when the clone master re-syncs")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::CloneImmune, id);
            ui.close();
        }
        let active = app.graph.node(id).flags.viewer_active;
        if ui
            .selectable_label(active, "Viewer Active")
            .on_hover_text("Operate the node viewer — A")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::ViewerActive, id);
            ui.close();
        }
        if ui.button("Delete").clicked() {
            app.queue_delete_selection();
            ui.close();
        }
    });
}