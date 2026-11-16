use egui::{Color32, RichText, Ui};
use otd_core::NodeId;

use crate::app::OtdApp;

use super::labels::op_display_label;

pub fn inspector_header_title(app: &OtdApp, id: NodeId) -> String {
    op_display_label(app, id)
}

pub fn header_bar(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    let family = app.graph.node(id).family;
    let c = family.color();
    let base = Color32::from_rgb(c[0], c[1], c[2]);
    let header_bg = base.gamma_multiply(0.38);
    let name_bg = base.gamma_multiply(0.52);
    let title = inspector_header_title(app, id);
    let mut name = app.graph.node(id).name.clone();

    egui::Frame::new()
        .fill(header_bg)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .strong()
                        .color(Color32::from_rgb(240, 240, 245)),
                );
                ui.add_space(6.0);
                egui::Frame::new()
                    .fill(name_bg)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut name)
                                    .desired_width(ui.available_width().max(80.0))
                                    .frame(egui::Frame::NONE),
                            )
                            .changed()
                        {
                            app.try_set_node_name(id, &name);
                        }
                    });
            });
        });

    ui.add_space(2.0);
}