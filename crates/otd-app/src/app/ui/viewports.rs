use std::sync::atomic::Ordering;

use super::*;

impl OtdApp {
    pub(super) fn output_viewport(&mut self, ctx: &egui::Context) {
        if self.perform_shell.output_closed.swap(false, Ordering::Relaxed) {
            self.perform_shell.output_window = false;
        }
        if !self.perform_shell.output_window {
            return;
        }
        let tex = self.perform_output_thumbnail();
        let closed = self.perform_shell.output_closed.clone();
        let builder = egui::ViewportBuilder::default()
            .with_title("OpenTouchDesigner — Output")
            .with_inner_size([1280.0, 720.0])
            .with_fullscreen(self.perform_shell.output_fullscreen);

        ctx.show_viewport_deferred(
            egui::ViewportId::from_hash_of("otd-output"),
            builder,
            move |ui, _class| {
                egui::CentralPanel::no_frame()
                    .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
                    .show(ui, |ui| {
                        if let Some((tid, size)) = tex {
                            let lb = letterbox(ui, tid, size);
                            draw_panel_texture(ui.painter(), lb, tid, size);
                        }
                    });
                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    closed.store(true, Ordering::Relaxed);
                }
            },
        );
    }
}
