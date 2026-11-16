use super::super::*;

impl OtdApp {
    pub(crate) fn close_save_dialog(&mut self, ctx: &egui::Context) {
        if !self.document.show_close_save_prompt {
            return;
        }
        let mut save = false;
        let mut discard = false;
        let mut dismiss = false;
        egui::Window::new("Save changes?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Save changes before closing?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                    if ui.button("Don't Save").clicked() {
                        discard = true;
                    }
                    if ui.button("Cancel").clicked() {
                        dismiss = true;
                    }
                });
            });
        if save {
            let path = self.document.project_path.clone();
            self.save(path);
            if !self.document.document_dirty {
                self.document.show_close_save_prompt = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if discard {
            self.document.document_dirty = false;
            self.document.show_close_save_prompt = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if dismiss {
            self.document.show_close_save_prompt = false;
        }
    }

    pub(crate) fn file_keys(&mut self, ctx: &egui::Context) {
        let (save, save_as) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                cmd && !i.modifiers.shift && i.key_pressed(egui::Key::S),
                cmd && i.modifiers.shift && i.key_pressed(egui::Key::S),
            )
        });
        if save_as {
            self.save_as();
        } else if save {
            let path = self.document.project_path.clone();
            self.save(path);
        }
    }

    pub(crate) fn handle_main_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) {
            return;
        }
        if self.document.show_close_save_prompt || self.file_dialogs.is_open() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            return;
        }
        if self.settings.prompt_save_on_exit && self.document.document_dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.document.show_close_save_prompt = true;
        }
    }

    pub(crate) fn history_keys(&mut self, ctx: &egui::Context) {
        if ctx.text_edit_focused() {
            return;
        }
        let (undo, redo) = ctx.input(|i| {
            let cmd = i.modifiers.command;
            (
                cmd && !i.modifiers.shift && i.key_pressed(egui::Key::Z),
                cmd && ((i.modifiers.shift && i.key_pressed(egui::Key::Z))
                    || i.key_pressed(egui::Key::Y)),
            )
        });
        if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
    }

    pub(crate) fn navigation_keys(&mut self, ctx: &egui::Context) {
        if self.perform_preview_only() || ctx.text_edit_focused() {
            return;
        }
        let mut leave = false;
        let mut enter = None;
        ctx.input_mut(|i| {
            if i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt {
                return;
            }
            if i.consume_key(egui::Modifiers::NONE, egui::Key::U) {
                leave = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::I)
            {
                enter = self.canvas.selected.filter(|id| {
                    self.graph.contains(*id)
                        && self.graph.node(*id).family == otd_core::Family::Comp
                });
            } else if self.settings.enable_playbar_shortcuts
                && i.consume_key(egui::Modifiers::NONE, egui::Key::Space)
            {
                self.transport.playing = !self.transport.playing;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::P)
                && crate::shortcut_routing::inspector_shortcut_context(self, ctx)
            {
                self.show_params_panel = !self.show_params_panel;
            }
        });
        if leave {
            self.queue_leave_network();
        } else if let Some(id) = enter {
            self.queue_enter_network(id);
        }
    }
}