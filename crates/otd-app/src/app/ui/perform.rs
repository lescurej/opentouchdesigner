use super::*;

impl OtdApp {
    pub(super) fn perform_preview_hint(&self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let pad = egui::vec2(12.0, 10.0);
        let pos = rect.left_bottom() + pad;
        let text = "Esc or E — edit Perform UI    F1 — preview";
        ui.painter().text(
            pos,
            egui::Align2::LEFT_BOTTOM,
            text,
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgba_unmultiplied(200, 204, 212, 180),
        );
    }

    pub(super) fn perform_edit_banner(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("perform_edit_banner").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Perform UI edit")
                        .strong()
                        .color(egui::Color32::from_rgb(220, 200, 140)),
                );
                if ui.button("Preview (F1)").clicked() {
                    self.perform_shell.perform_edit = false;
                }
                if ui.button("Exit perform").clicked() {
                    self.leave_perform_for_edit();
                }
            });
        });
    }

    /// Perform preview: fullscreen **View** of the perform-ui containerCOMP.
    ///
    /// Same path as `view_panel_comp`: recursive container layout, leaf panel
    /// TOPs, arbitrary TOPs only via opviewerCOMP.
    pub(super) fn perform_view(&mut self, ui: &mut egui::Ui) {
        let perform_comp = self.perform_ui();
        let widgets = perform_comp
            .map(|c| otd_engine::panel::panel_display_widgets(&self.graph, c))
            .unwrap_or_default();
        let mut panels: Vec<(NodeId, f64)> = Vec::new();
        self.canvas.visible.clear();

        egui::CentralPanel::no_frame()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ui, |ui| {
                let area = ui.max_rect();
                match perform_comp {
                    Some(comp) if !widgets.is_empty() => {
                        panels.extend(draw_panel_comp(
                            ui, self, comp, area, &widgets, true,
                        ));
                    }
                    Some(comp) => {
                        if let Some(top) =
                            crate::canvas::comp_display::comp_view_top(&self.graph, comp)
                        {
                            if let Some((tid, size)) = self.thumbnail(top) {
                                let lb = letterbox(ui, tid, size);
                                super::panel_draw::draw_panel_comp_look(&self.graph, comp, ui.painter(), lb);
                                let w = otd_engine::panel::panel_display_widgets(&self.graph, comp);
                                panels.extend(draw_panel(ui, &w, lb, Some(self), true));
                            }
                        } else {
                            ui.centered_and_justified(|ui| {
                                ui.label(
                                    egui::RichText::new("Panel has no cooked view — open the COMP and wire a view TOP")
                                        .color(egui::Color32::from_gray(90)),
                                );
                            });
                        }
                    }
                    None => {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new("no Perform UI container — set one in Window Placement")
                                    .color(egui::Color32::from_gray(90)),
                            );
                        });
                    }
                }
            });

        // Applied after the panel is drawn, because the write needs the graph
        // mutably and drawing borrowed it.
        for (id, value) in panels {
            self.edit("panel");
            let _ = self
                .graph
                .set_param(id, "value", otd_core::Value::Float(value));
        }
    }
}
