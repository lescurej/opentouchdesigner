use super::*;

impl eframe::App for OtdApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, _raw_input: &mut egui::RawInput) {
        self.sync_pending_textures(ctx);
        if self.transport.playing {
            self.sync_playing_display_blits(ctx);
        }
        self.drain_graph_commands();
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if ctx.current_pass_index() > 0 {
            return;
        }
        crate::macos_open::install_once();
        self.run_pending_launch();
        crate::app::file_dialog::note_host(self, frame);
        let dialog_started = std::time::Instant::now();
        crate::app::file_dialog::poll(self, ctx);
        self.frame_perf_probe.poll_async_ms = dialog_started.elapsed().as_secs_f64() * 1000.0;
        crate::dat_external::poll_external_edits(self);
        #[cfg(target_os = "macos")]
        self.poll_macos_open_files();
        self.sync_viewport_playback(ctx);
        let now = std::time::Instant::now();
        let gap_ms = now
            .duration_since(self.last_logic_at)
            .as_secs_f64()
            * 1000.0;
        self.frame_perf_probe.event_loop_gap_ms = gap_ms;
        self.transport.smoothed_wall_tick_ms = self.transport.smoothed_wall_tick_ms * 0.92 + gap_ms * 0.08;
        self.last_logic_at = now;
        self.handle_main_close_request(ctx);
        let minimized = ctx.input(|i| i.viewport().minimized == Some(true));
        let cook_start = std::time::Instant::now();
        if minimized && self.transport.playing {
            self.sync_background_cook(true);
        } else {
            self.background_engine.pause_for_ui();
            self.drain_background_reports();
            self.cook_frame(ctx, true, true);
        }
        self.frame_perf_probe.logic_ms = cook_start.elapsed().as_secs_f64() * 1000.0;
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ui_started = std::time::Instant::now();
        crate::shortcut_routing::begin_frame(self);
        if self.blur_params_on_select {
            self.blur_params_on_select = false;
            crate::canvas::input::release_param_focus(ui.ctx());
        }
        self.clipboard_keys(ui.ctx());

        self.create_operator_tab_key(ui.ctx());
        self.history_keys(ui.ctx());
        self.navigation_keys(ui.ctx());
        self.file_keys(ui.ctx());
        self.selection_keys(ui.ctx());
        self.flag_keys(ui.ctx());

        // F1 anywhere, including out of perform mode — a performer who cannot
        // find the way back out of a black screen has a real problem.
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F1)) {
            if self.perform_preview_only() {
                self.leave_perform_for_edit();
            } else if self.perform_shell.perform && self.perform_shell.perform_edit {
                self.perform_shell.perform_edit = false;
            } else {
                self.perform_shell.perform = true;
                self.perform_shell.perform_edit = false;
            }
        }
        if self.perform_preview_only()
            && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::E))
        {
            self.perform_shell.perform_edit = true;
            self.focus_perform_ui_for_edit();
        }
        if self.perform_shell.perform && self.perform_shell.perform_edit && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.leave_perform_for_edit();
        }
        let want_fullscreen = self.perform_preview_only();
        if want_fullscreen != self.perform_shell.perform_applied {
            self.perform_shell.perform_applied = want_fullscreen;
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Fullscreen(want_fullscreen));
        }
        // F2 records, in either mode — the top bar is not there in the one
        // where you most want to hit record.
        if ui.ctx().input(|i| i.key_pressed(egui::Key::F2)) {
            self.toggle_record();
        }
        if ui.ctx().input(|i| i.modifiers.alt && i.key_pressed(egui::Key::T)) {
            self.show_textport = !self.show_textport;
        }
        if ui.ctx().input(|i| i.modifiers.alt && i.key_pressed(egui::Key::W)) {
            self.perform_shell.show_window_placement = !self.perform_shell.show_window_placement;
        }
        if self.perform_shell.designer_position_pending {
            self.perform_shell.designer_position_pending = false;
            self.apply_designer_window_position(ui.ctx());
        }

        if self.perform_preview_only() {
            self.perform_view(ui);
            self.perform_preview_hint(ui);
        } else {
            if self.perform_shell.perform && self.perform_shell.perform_edit {
                self.perform_edit_banner(ui);
            }
            self.top_bar(ui);
            self.status_bar(ui);
            self.monitor(ui.ctx());
            crate::textport::Textport::show(self, ui.ctx());
            self.timeline(ui);
            self.side_panel(ui);
            let canvas_start = std::time::Instant::now();
            crate::canvas::show(self, ui);
            self.frame_perf_probe.canvas_ms = canvas_start.elapsed().as_secs_f64() * 1000.0;
            self.clipboard_keys(ui.ctx());
        }
        crate::assistant::bar(self, ui.ctx());
        crate::assistant::window(self, ui.ctx());
        if !self.perform_preview_only() {
            crate::create_dialog::show(self, ui);
        }
        self.output_viewport(ui.ctx());
        self.window_comp_viewports(ui.ctx());
        if !self.perform_preview_only() {
            crate::container_view::show(self, ui.ctx());
        }
        crate::window_placement::show(self, ui.ctx());
        crate::settings::show(self, ui.ctx());
        crate::td_import::show(self, ui.ctx());
        self.close_save_dialog(ui.ctx());

        // Files can be dropped in either mode: a performer swapping a clip
        // mid-show should not have to leave the output to do it.
        crate::media::hover_overlay(self, ui.ctx());
        crate::media::handle_drops(self, ui.ctx());

        self.frame_perf_probe.ui_ms = ui_started.elapsed().as_secs_f64() * 1000.0;
        ui.ctx().request_repaint();
    }
}
