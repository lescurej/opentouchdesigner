use super::*;

impl OtdApp {
    pub(super) fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close();
                    }
                    if ui.button("Open…").clicked() {
                        self.open(None);
                        ui.close();
                    }
                    if ui
                        .add(egui::Button::new("Save").shortcut_text("Cmd+S"))
                        .clicked()
                    {
                        let p = self.document.project_path.clone();
                        self.save(p);
                        ui.close();
                    }
                    if ui
                        .add(egui::Button::new("Save As…").shortcut_text("Cmd+Shift+S"))
                        .clicked()
                    {
                        self.save_as();
                        ui.close();
                    }
                    if ui
                        .button("Export Bundle…")
                        .on_hover_text(
                            "Copy the project and every component it uses into one folder, \
                             ready to move to a show machine",
                        )
                        .clicked()
                    {
                        self.export_bundle();
                        ui.close();
                    }
                    ui.separator();
                    let on_comp = self
                        .canvas.selected
                        .and_then(|s| self.graph.get(s))
                        .map(|n| n.family == otd_core::Family::Comp)
                        .unwrap_or(false);
                    if ui
                        .add_enabled(on_comp, egui::Button::new("Save Component As…"))
                        .on_hover_text("Write the selected component to a .otdc file")
                        .clicked()
                    {
                        self.save_component();
                        ui.close();
                    }
                    if ui
                        .button("Import Component…")
                        .on_hover_text("Load a .otdc into this network, linked to its file")
                        .clicked()
                    {
                        self.import_component();
                        ui.close();
                    }
                    ui.separator();
                    ui.menu_button("Examples", |ui| {
                        for name in otd_engine::demo::NAMES {
                            if ui.button(*name).clicked() {
                                self.load_demo(name);
                                ui.close();
                            }
                        }
                    });
                    ui.menu_button("Palette", |ui| {
                        for item in otd_engine::palette::ITEMS {
                            if ui.button(item.name).on_hover_text(item.summary).clicked() {
                                self.edit("palette");
                                let id = item.build(&mut self.graph, &self.registry, self.canvas.current);
                                self.select_only(id);
                                self.document.status = format!("added {} from the palette", item.name);
                                ui.close();
                            }
                        }
                    });
                });
                ui.menu_button("Edit", |ui| {
                    let (back, ahead) = self.document.history.depth();
                    let has_sel = !self.canvas.selection.is_empty() || self.canvas.selected.is_some();
                    let can_paste = !self.document.node_clipboard.nodes.is_empty();
                    if ui
                        .add_enabled(back > 0, egui::Button::new("Undo").shortcut_text("Cmd+Z"))
                        .clicked()
                    {
                        self.undo();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            ahead > 0,
                            egui::Button::new("Redo").shortcut_text("Cmd+Shift+Z"),
                        )
                        .clicked()
                    {
                        self.redo();
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Copy").shortcut_text("Cmd+C"))
                        .clicked()
                    {
                        self.copy_selection(ui.ctx());
                        ui.close();
                    }
                    if ui
                        .add_enabled(has_sel, egui::Button::new("Cut").shortcut_text("Cmd+X"))
                        .clicked()
                    {
                        self.cut_selection(ui.ctx());
                        ui.close();
                    }
                    if ui
                        .add_enabled(can_paste, egui::Button::new("Paste").shortcut_text("Cmd+V"))
                        .clicked()
                    {
                        let ctx = ui.ctx();
                        let pointer = ctx
                            .pointer_latest_pos()
                            .unwrap_or_else(|| self.canvas.canvas_rect.center());
                        let world = self.canvas.view.to_world(self.canvas.canvas_rect.min, pointer);
                        self.queue_paste_clipboard([world.x, world.y]);
                        ui.close();
                    }
                });
                self.media_menu(ui);
                ui.menu_button("Dialogs", |ui| {
                    if ui.button("Settings…").clicked() {
                        self.show_settings = true;
                        ui.close();
                    }
                    if ui
                        .add(egui::Button::new("Window Placement…").shortcut_text("Alt+W"))
                        .clicked()
                    {
                        self.perform_shell.show_window_placement = true;
                        ui.close();
                    }
                });

                // Undo is on the bar, not only in the menu and the shortcut.
                // It is the one thing reached for in a hurry and while looking
                // at the network rather than at the menu, and a step you
                // cannot see how to take is a step you do not take. Disabled
                // when there is nothing to undo, so the bar also answers
                // "is there anything to go back to".
                ui.separator();
                let (back, ahead) = self.document.history.depth();
                let undo = ui.push_id(egui::Id::new("otd_topbar_undo"), |ui| {
                    ui.add_enabled(back > 0, egui::Button::new("↶"))
                });
                if undo.inner.on_hover_text("Undo (Cmd+Z)").clicked() {
                    self.undo();
                }
                let redo = ui.push_id(egui::Id::new("otd_topbar_redo"), |ui| {
                    ui.add_enabled(ahead > 0, egui::Button::new("↷"))
                });
                if redo.inner.on_hover_text("Redo (Cmd+Shift+Z)").clicked() {
                    self.redo();
                }

                ui.separator();
                let icon = if self.transport.playing { "⏸" } else { "▶" };
                if ui
                    .push_id(egui::Id::new("otd_topbar_play"), |ui| {
                        ui.button(icon).on_hover_text("Play / pause time")
                    })
                    .inner
                    .clicked()
                {
                    self.transport.playing = !self.transport.playing;
                }
                if ui
                    .push_id(egui::Id::new("otd_topbar_reset_time"), |ui| {
                        ui.button("⟲").on_hover_text("Reset time to 0")
                    })
                    .inner
                    .clicked()
                {
                    self.time.frame = 0;
                    self.time.time = 0.0;
                    self.time.abs_time = 0.0;
                }
                ui.push_id(egui::Id::new("otd_topbar_time"), |ui| {
                    ui.label(format!(
                        "frame {}   {:.2}s",
                        self.time.frame, self.time.abs_time
                    ));
                });

                ui.separator();
                // Breadcrumb: click a step to go back to it.
                let mut chain = Vec::new();
                let mut cur = Some(self.canvas.current);
                while let Some(c) = cur {
                    chain.push(c);
                    cur = self.graph.get(c).and_then(|n| n.parent);
                }
                chain.reverse();
                for (i, id) in chain.iter().enumerate() {
                    if i > 0 {
                        ui.push_id((*id, "otd_bc_sep"), |ui| {
                            ui.label(egui::RichText::new("/").weak());
                        });
                    }
                    let name = if *id == self.graph.root() {
                        "/".to_string()
                    } else {
                        self.graph
                            .get(*id)
                            .map(|n| n.name.clone())
                            .unwrap_or_else(|| "?".into())
                    };
                    let last = i + 1 == chain.len();
                    let text = if last {
                        egui::RichText::new(name).strong()
                    } else {
                        egui::RichText::new(name).weak()
                    };
                    let nav = *id;
                    if ui.push_id((*id, "otd_bc"), |ui| ui.selectable_label(false, text)).inner.clicked()
                    {
                        self.switch_network(nav);
                        self.clear_selection();
                    }
                }

                ui.separator();
                if ui
                    .push_id(egui::Id::new("otd_topbar_perform"), |ui| {
                        ui.button("Perform")
                            .on_hover_text("Hide the editor and show only the output (F1)")
                    })
                    .inner
                    .clicked()
                {
                    self.perform_shell.perform = true;
                    self.perform_shell.perform_edit = false;
                }
                let recording = self.recording();
                let label = if recording { "⏹ Stop" } else { "⏺ Record" };
                if ui
                    .push_id(egui::Id::new("otd_topbar_record"), |ui| {
                        ui.button(label).on_hover_text("Record the viewer to a movie file (F2)")
                    })
                    .inner
                    .clicked()
                {
                    self.toggle_record();
                }
                ui.push_id(egui::Id::new("otd_topbar_assistant"), |ui| {
                    ui.toggle_value(&mut self.assistant.bar, "✨ Assistant")
                        .on_hover_text("Describe a patch and have it built here — Cmd/Ctrl+K");
                });
                ui.push_id(egui::Id::new("otd_topbar_perf_toggle"), |ui| {
                    ui.toggle_value(&mut self.show_monitor, "Perf")
                        .on_hover_text("Per-node cook cost and GPU memory");
                });
                ui.push_id(egui::Id::new("otd_topbar_textport_toggle"), |ui| {
                    ui.toggle_value(&mut self.show_textport, "Textport")
                        .on_hover_text("Log and Python console (Alt+T)");
                });
                ui.push_id(egui::Id::new("otd_topbar_output_toggle"), |ui| {
                    ui.toggle_value(&mut self.perform_shell.output_window, "Output")
                        .on_hover_text("Open a second window showing only the viewer");
                });
                ui.push_id(egui::Id::new("otd_topbar_output_fullscreen"), |ui| {
                    if self.perform_shell.output_window {
                        ui.toggle_value(&mut self.perform_shell.output_fullscreen, "Fullscreen");
                    }
                });
            });
        });
    }

    /// Your own material, and the things to do to it.
    ///
    /// Everything here is reachable another way — Tab finds any operator, and
    /// a file parameter has a Browse button. It is a menu because the other
    /// ways all require knowing the name of an operator first.
    fn media_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Media", |ui| {
            if ui
                .button("Import Media…")
                .on_hover_text("Movies, images and audio — or just drop the files on the network")
                .clicked()
            {
                crate::media::import_dialog(self);
                ui.close();
            }
            if ui
                .button("Use Webcam")
                .on_hover_text("A Video Device In, viewing straight away")
                .clicked()
            {
                crate::media::add_webcam(self);
                ui.close();
            }
            ui.separator();

            let selected = self
                .canvas.selected
                .filter(|s| self.graph.contains(*s))
                .map(|s| (self.graph.node(s).name.clone(), self.graph.node(s).family));
            let label = match &selected {
                Some((name, family)) => format!("Add {} after {name}", family.suffix()),
                None => "Add operator".to_string(),
            };
            let only_family = selected.as_ref().map(|(_, f)| *f);
            ui.add_enabled_ui(selected.is_some(), |ui| {
                ui.menu_button(label, |ui| {
                    crate::media::add_operator_menu(self, ui, only_family);
                });
            });
            if selected.is_none() {
                ui.label(
                    egui::RichText::new("select an operator to add downstream")
                        .weak()
                        .small(),
                );
            }
        });
    }
}
