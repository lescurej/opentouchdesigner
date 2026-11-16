use super::*;

impl OtdApp {
    pub(super) fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("statusbar")
            .exact_size(28.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.push_id(egui::Id::new("otd_status_fps"), |ui| {
                        ui.label(format!("{:.0} fps", self.transport.smoothed_fps));
                    });
                    ui.separator();
                    ui.push_id(egui::Id::new("otd_status_cook"), |ui| {
                        ui.label(format!(
                            "cook {:.2} ms  ({} cooked / {} cached)",
                            self.transport.smoothed_cook_ms,
                            self.last_frame_timing.cooked,
                            self.last_frame_timing.cached
                        ));
                    });
                    ui.separator();
                    let (passes, megabytes) = {
                        let rt = self.engine_rt();
                        let bytes =
                            rt.engines.top.resident_bytes() + rt.engines.top.pooled_bytes();
                        (rt.engines.top.passes_this_frame, bytes as f64 / 1.0e6)
                    };
                    ui.push_id(egui::Id::new("otd_status_gpu"), |ui| {
                        ui.label(format!("{passes} passes  {megabytes:.0} MB"));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (text, color) = match &self.document.cook_error {
                            Some(err) => (err.as_str(), egui::Color32::from_rgb(230, 110, 110)),
                            None => (&self.document.status as &str, ui.style().visuals.text_color()),
                        };
                        ui.push_id(egui::Id::new("otd_status"), |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(text).color(color)).truncate(),
                            )
                            .on_hover_text(text);
                        });
                    });
                });
            });
    }
}
