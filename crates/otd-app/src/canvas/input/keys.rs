use egui::{Pos2, Rect};

use crate::app::OtdApp;
use crate::canvas::types::DragState;

use super::clipboard::operator_clipboard_keys;
use super::view::{frame_all_nodes, release_param_focus};

pub(crate) fn handle_keys(app: &mut OtdApp, ui: &egui::Ui, _origin: Pos2, _rect: Rect) {
    operator_clipboard_keys(app, ui.ctx());

    let ctx = ui.ctx();
    if crate::shortcut_routing::network_owns_keyboard_focus(app, ctx) {
        let mut select_all = false;
        let mut delete = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::A) {
                select_all = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
            {
                delete = true;
            }
        });
        if select_all {
            release_param_focus(ctx);
            app.select_all();
        } else if delete {
            release_param_focus(ctx);
            app.queue_delete_selection();
        }
    }

    if ui.ctx().input(|i| i.pointer.any_released()) {
        if let Some(DragState::Node { id, .. }) = app.canvas.drag {
            let ids = crate::canvas::node::node_drag_group(app, id);
            for nid in ids {
                if app.graph.contains(nid) && !app.graph.node(nid).flags.lock {
                    let p = app.graph.node(nid).pos;
                    app.graph.node_mut_quiet(nid).pos = app.snap_world_pos(p);
                }
            }
            app.canvas.node_drag_start = None;
            app.canvas.drag = None;
            app.document.history.end_gesture();
        }
    }

    if crate::shortcut_routing::network_owns_keyboard_focus(app, ui.ctx()) {
        ui.ctx().input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::H) {
                frame_all_nodes(app);
            }
        });
    }
}
