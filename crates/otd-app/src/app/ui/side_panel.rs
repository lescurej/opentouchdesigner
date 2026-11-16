use super::*;

impl OtdApp {
    pub(super) fn side_panel(&mut self, ui: &mut egui::Ui) {
        if !self.show_params_panel {
            return;
        }
        let resp = egui::Panel::right("params")
            .default_size(400.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::params::show(self, ui);
                });
            });
        self.params_panel_rect = resp.response.rect;
    }
}
