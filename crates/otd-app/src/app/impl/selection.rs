use super::super::*;

impl OtdApp {
    pub fn clear_selection(&mut self) {
        self.canvas.selection.clear();
        self.canvas.selected = None;
        self.canvas.selected_wire = None;
    }

    pub fn is_selected(&self, id: NodeId) -> bool {
        self.canvas.selection.contains(&id)
    }

    pub fn select_all(&mut self) {
        self.canvas.selection = self.graph.node(self.canvas.current).children.iter().copied().collect();
        if !self.canvas.selected.map(|s| self.canvas.selection.contains(&s)).unwrap_or(false) {
            self.canvas.selected = self.canvas.selection.iter().next().copied();
        }
    }

    pub fn select_only(&mut self, id: NodeId) {
        self.canvas.selection.clear();
        self.canvas.selection.insert(id);
        self.canvas.selected = Some(id);
        self.canvas.selected_wire = None;
        self.blur_params_on_select = self.settings.blur_params_on_select;
    }

    pub fn select_toggle(&mut self, id: NodeId) {
        if self.canvas.selection.remove(&id) {
            if self.canvas.selected == Some(id) {
                self.canvas.selected = self.canvas.selection.iter().next().copied();
            }
        } else {
            self.canvas.selection.insert(id);
            self.canvas.selected = Some(id);
        }
    }

    pub(crate) fn flag_keys(&mut self, ctx: &egui::Context) {
        if self.perform_preview_only() {
            return;
        }
        let count = self.canvas.selection.len();
        if count == 0 && self.canvas.selected.is_none() {
            return;
        }
        if ctx.text_edit_focused() && count <= 1 {
            return;
        }
        let flag = ctx.input_mut(|i| {
            if i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt {
                return None;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::V) {
                Some(crate::canvas::input::SelectionFlag::Viewer)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::D) {
                Some(crate::canvas::input::SelectionFlag::Display)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::L) {
                Some(crate::canvas::input::SelectionFlag::Lock)
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::B) {
                Some(crate::canvas::input::SelectionFlag::Bypass)
            } else {
                None
            }
        });
        if let Some(flag) = flag {
            crate::canvas::input::toggle_flag_on_selection(self, flag);
        }
    }

    pub(crate) fn selection_keys(&mut self, ctx: &egui::Context) {
        let cmd_a = ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A));
        if !cmd_a {
            return;
        }
        let over_network = ctx
            .pointer_latest_pos()
            .map(|p| self.canvas.canvas_rect.contains(p))
            .unwrap_or(true);
        if ctx.text_edit_focused() && !over_network {
            return;
        }
        if !ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::A)) {
            return;
        }
        crate::canvas::input::release_param_focus(ctx);
        self.select_all();
    }
}