use egui::{RichText, Ui};
use otd_core::NodeId;

use crate::app::OtdApp;
use crate::window_comp::{is_open, list_perform_ui_containers, list_window_comps};

pub fn show(app: &mut OtdApp, ctx: &egui::Context) {
    if !app.show_window_placement {
        return;
    }
    let mut open = app.show_window_placement;
    egui::Window::new("Window Placement")
        .open(&mut open)
        .default_width(720.0)
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Manage Window COMPs — same controls as each operator's Window page.")
                    .weak()
                    .small(),
            );
            ui.separator();

            ui.checkbox(&mut app.start_in_perform_mode, "Start in Perform Mode")
                .on_hover_text("Open the project in perform mode (F1) instead of the editor");

            ui.separator();
            ui.strong("Perform UI");
            ui.label(
                RichText::new("F1 shows this container COMP — panel layout and cooked child TOPs.")
                    .weak()
                    .small(),
            );
            egui::ScrollArea::vertical()
                .id_salt("window_placement_perform_ui")
                .max_height(160.0)
                .show(ui, |ui| {
                    let containers = list_perform_ui_containers(&app.graph);
                    if containers.is_empty() {
                        ui.label(
                            RichText::new("No containerCOMP — add one and put button/slider COMPs inside.")
                                .weak(),
                        );
                    } else {
                        ui.horizontal(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(220.0, 18.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| ui.label(RichText::new("Container").strong()),
                            );
                            ui.label(RichText::new("Perform UI").strong());
                        });
                        ui.separator();
                        for id in containers {
                            perform_ui_row(app, ui, id);
                        }
                    }
                });

            ui.separator();
            ui.strong("Window components");
            egui::ScrollArea::vertical()
                .id_salt("window_placement_window_comps")
                .max_height(220.0)
                .show(ui, |ui| {
                    let windows = list_window_comps(&app.graph);
                    if windows.is_empty() {
                        ui.label(
                            RichText::new("No windowCOMP in this project — add one from Create → COMP.")
                                .weak(),
                        );
                    } else {
                        table_header(ui);
                        for id in windows {
                            window_row(app, ui, id);
                        }
                    }
                });

            ui.separator();
            ui.strong("Designer window");
            ui.checkbox(
                &mut app.designer_custom_position,
                "Custom position and size",
            )
            .on_hover_text("Where the main editor window opens (designer mode)");
            ui.horizontal(|ui| {
                ui.label("X");
                ui.add(
                    egui::DragValue::new(&mut app.designer_x)
                        .speed(1.0)
                        .range(-8192.0..=8192.0),
                );
                ui.label("Y");
                ui.add(
                    egui::DragValue::new(&mut app.designer_y)
                        .speed(1.0)
                        .range(-8192.0..=8192.0),
                );
                ui.label("W");
                ui.add(
                    egui::DragValue::new(&mut app.designer_w)
                        .speed(1.0)
                        .range(400.0..=8192.0),
                );
                ui.label("H");
                ui.add(
                    egui::DragValue::new(&mut app.designer_h)
                        .speed(1.0)
                        .range(300.0..=8192.0),
                );
            });
            ui.horizontal(|ui| {
                if ui.button("Apply to designer window").clicked() {
                    app.apply_designer_window_position(ctx);
                }
                if ui
                    .button("Use current window")
                    .on_hover_text("Copy the main window's position and size into these fields")
                    .clicked()
                {
                    app.capture_designer_window_position(ctx);
                }
            });
        });
    app.show_window_placement = open;
}

fn table_header(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(200.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| ui.label(RichText::new("Path").strong()),
        );
        ui.label(RichText::new("Output window").strong());
        ui.label(RichText::new("Open").strong());
        ui.label(RichText::new("").strong());
    });
    ui.separator();
}

fn perform_ui_row(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    let path = app.graph.path(id);
    let name = app.graph.node(id).name.clone();
    let selected = app.perform_ui() == Some(id);

    ui.push_id(egui::Id::new(("wp_perform_ui_row", id)), |ui| {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(220.0, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new(format!("{path}  ({name})")).small());
                },
            );
            let radio = ui.push_id("radio", |ui| {
                ui.add(egui::RadioButton::new(selected, ""))
                    .on_hover_text("Use this container for perform mode (F1)")
            });
            if radio.inner.clicked() && !selected {
                app.set_perform_ui(id);
            }
            if ui
                .button("P")
                .on_hover_text("Edit parameters (opens container network)")
                .clicked()
            {
                app.perform_edit = false;
                app.perform = false;
                app.enter(id);
                app.select_only(id);
            }
            if ui.button("→").on_hover_text("Open this network").clicked() {
                app.perform_edit = false;
                app.perform = false;
                app.enter(id);
                app.select_only(id);
            }
        });
    });
}

fn window_row(app: &mut OtdApp, ui: &mut Ui, id: NodeId) {
    let path = app.graph.path(id);
    let name = app.graph.node(id).name.clone();
    let open = is_open(&app.graph, id);
    let main = app.main_perform_window == Some(id);

    ui.push_id(egui::Id::new(("wp_window_row", id)), |ui| {
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(200.0, 22.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new(format!("{path}  ({name})")).small());
                },
            );

            let radio = ui.push_id("radio", |ui| {
                ui.add(egui::RadioButton::new(main, "")).on_hover_text(
                    "Default windowCOMP for projector / output TOP (winop → cooked TOP only)",
                )
            });
            if radio.inner.clicked() && !main {
                app.set_main_perform_window(id);
            }

            if open {
                if ui.button("Close").clicked() {
                    app.set_window_open(id, false);
                }
            } else if ui.button("Open").clicked() {
                app.set_window_open(id, true);
            }

            if ui
                .button("P")
                .on_hover_text("Select and show parameters")
                .clicked()
            {
                app.select_only(id);
            }
            if ui
                .button("→")
                .on_hover_text("Open the network containing this Window COMP")
                .clicked()
            {
                if let Some(parent) = app.graph.node(id).parent {
                    app.current = parent;
                    app.select_only(id);
                }
            }
        });
    });
}
