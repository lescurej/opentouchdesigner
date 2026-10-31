use super::*;

impl eframe::App for OtdApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, _raw_input: &mut egui::RawInput) {
        self.sync_pending_textures();
        self.drain_graph_commands();
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.current_pass_index() > 0 {
            return;
        }
        self.sync_viewport_playback(ctx);
        self.handle_main_close_request(ctx);
        self.cook_frame();
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
            } else if self.perform && self.perform_edit {
                self.perform_edit = false;
            } else {
                self.perform = true;
                self.perform_edit = false;
            }
        }
        if self.perform_preview_only()
            && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::E))
        {
            self.perform_edit = true;
            self.focus_perform_ui_for_edit();
        }
        if self.perform && self.perform_edit && ui.ctx().input(|i| i.key_pressed(egui::Key::Escape))
        {
            self.leave_perform_for_edit();
        }
        let want_fullscreen = self.perform_preview_only();
        if want_fullscreen != self.perform_applied {
            self.perform_applied = want_fullscreen;
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
            self.show_window_placement = !self.show_window_placement;
        }
        if self.designer_position_pending {
            self.designer_position_pending = false;
            self.apply_designer_window_position(ui.ctx());
        }

        if self.perform_preview_only() {
            self.perform_view(ui);
            self.perform_preview_hint(ui);
        } else {
            if self.perform && self.perform_edit {
                self.perform_edit_banner(ui);
            }
            self.top_bar(ui);
            self.monitor(ui.ctx());
            crate::textport::Textport::show(self, ui.ctx());
            self.timeline(ui);
            self.side_panel(ui);
            crate::canvas::show(self, ui);
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
        self.close_save_dialog(ui.ctx());

        // Files can be dropped in either mode: a performer swapping a clip
        // mid-show should not have to leave the output to do it.
        crate::media::hover_overlay(self, ui.ctx());
        crate::media::handle_drops(self, ui.ctx());

        ui.ctx().request_repaint();
    }
}

impl OtdApp {
    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Edit", |ui| {
                    let (back, ahead) = self.history.depth();
                    let has_sel = !self.selection.is_empty() || self.selected.is_some();
                    let can_paste = !self.node_clipboard.nodes.is_empty();
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
                            .unwrap_or_else(|| self.canvas_rect.center());
                        let world = self.view.to_world(self.canvas_rect.min, pointer);
                        self.queue_paste_clipboard([world.x, world.y]);
                        ui.close();
                    }
                });
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
                        let p = self.project_path.clone();
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
                        .selected
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
                                let id = item.build(&mut self.graph, &self.registry, self.current);
                                self.select_only(id);
                                self.status = format!("added {} from the palette", item.name);
                                ui.close();
                            }
                        }
                    });
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
                        self.show_window_placement = true;
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
                let (back, ahead) = self.history.depth();
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
                let icon = if self.playing { "⏸" } else { "▶" };
                if ui
                    .push_id(egui::Id::new("otd_topbar_play"), |ui| {
                        ui.button(icon).on_hover_text("Play / pause time")
                    })
                    .inner
                    .clicked()
                {
                    self.playing = !self.playing;
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
                let mut cur = Some(self.current);
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
                    self.perform = true;
                    self.perform_edit = false;
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
                    ui.toggle_value(&mut self.output_window, "Output")
                        .on_hover_text("Open a second window showing only the viewer");
                });
                ui.push_id(egui::Id::new("otd_topbar_output_fullscreen"), |ui| {
                    if self.output_window {
                        ui.toggle_value(&mut self.output_fullscreen, "Fullscreen");
                    }
                });

                ui.separator();
                ui.push_id(egui::Id::new("otd_topbar_fps"), |ui| {
                    ui.label(format!("{:.0} fps", self.smoothed_fps));
                });
                ui.push_id(egui::Id::new("otd_topbar_cook"), |ui| {
                    ui.label(format!(
                        "cook {:.2} ms  ({} cooked / {} cached)",
                        self.smoothed_cook_ms, self.cook.stats.cooked, self.cook.stats.cached
                    ));
                });
                ui.push_id(egui::Id::new("otd_topbar_gpu"), |ui| {
                    ui.label(format!(
                        "{} passes  {:.0} MB",
                        self.engines.top.passes_this_frame,
                        (self.engines.top.resident_bytes() + self.engines.top.pooled_bytes())
                            as f64
                            / 1.0e6
                    ));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (text, color) = match &self.cook_error {
                        Some(err) => (
                            err.as_str(),
                            egui::Color32::from_rgb(230, 110, 110),
                        ),
                        None => (&self.status as &str, ui.style().visuals.text_color()),
                    };
                    ui.push_id(egui::Id::new("otd_status"), |ui| {
                        ui.label(egui::RichText::new(text).color(color));
                    });
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
                .selected
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

    /// The performance monitor: what each node costs, and what the GPU holds.
    ///
    /// Ranked by cost *per frame* rather than by cook time. Those are not the
    /// same number in a demand-driven engine, and ranking by the wrong one
    /// sends you off optimising a node that cooks once and then sits there.
    fn monitor(&mut self, ctx: &egui::Context) {
        if !self.show_monitor {
            return;
        }
        let mut open = self.show_monitor;
        egui::Window::new("Performance")
            .open(&mut open)
            .default_width(420.0)
            .default_height(360.0)
            .show(ctx, |ui| {
                let budget = 1000.0 / self.time.fps.max(1.0);
                let frame_ms = 1000.0 / self.smoothed_fps.max(1e-6);
                ui.horizontal(|ui| {
                    ui.strong(format!("{frame_ms:.2} ms/frame"));
                    ui.label(
                        egui::RichText::new(format!("budget {budget:.2} ms"))
                            .weak()
                            .small(),
                    );
                    let (colour, verdict) = if frame_ms <= budget {
                        (egui::Color32::from_rgb(130, 200, 140), "holding the rate")
                    } else {
                        (egui::Color32::from_rgb(235, 120, 120), "over budget")
                    };
                    ui.colored_label(colour, egui::RichText::new(verdict).small());
                });
                ui.label(
                    egui::RichText::new(format!(
                        "cook {:.2} ms · {} cooked, {} cached last frame",
                        self.smoothed_cook_ms, self.cook.stats.cooked, self.cook.stats.cached
                    ))
                    .weak()
                    .small(),
                );

                let top = self.engines.top.pooled_bytes();
                let resident = self.engines.top.resident_bytes();
                ui.label(
                    egui::RichText::new(format!(
                        "GPU  {} in node outputs · {} pooled · {} textures created",
                        bytes(resident),
                        bytes(top),
                        self.engines.top.textures_created()
                    ))
                    .weak()
                    .small(),
                );
                ui.separator();

                // Every node that has cooked, most expensive per frame first.
                let mut rows: Vec<(NodeId, f64, f64, f64)> = self
                    .graph
                    .walk()
                    .into_iter()
                    .filter(|id| *id != self.graph.root())
                    .map(|id| {
                        (
                            id,
                            self.cook.frame_cost_ms(id),
                            self.cook.avg_cook_ms(id),
                            self.cook.cook_rate(id),
                        )
                    })
                    .filter(|(_, _, avg, _)| *avg > 0.0)
                    .collect();
                rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                if rows.is_empty() {
                    ui.label(egui::RichText::new("nothing has cooked yet").weak());
                    return;
                }
                let worst = rows[0].1.max(1e-6);

                let mut jump = None;
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("perf-rows")
                        .num_columns(4)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("node").small().strong());
                            ui.label(egui::RichText::new("ms/frame").small().strong());
                            ui.label(egui::RichText::new("per cook").small().strong());
                            ui.label(egui::RichText::new("cooks").small().strong());
                            ui.end_row();

                            for (id, cost, avg, rate) in &rows {
                                let path = self.graph.path(*id);
                                if ui.selectable_label(false, &path).clicked() {
                                    jump = Some(*id);
                                }
                                // Tint by share of the worst offender, so the
                                // eye lands on the problem without reading.
                                let heat = (cost / worst) as f32;
                                ui.colored_label(
                                    egui::Color32::from_rgb(
                                        (150.0 + 85.0 * heat) as u8,
                                        (200.0 - 80.0 * heat) as u8,
                                        (160.0 - 60.0 * heat) as u8,
                                    ),
                                    egui::RichText::new(format!("{cost:.3}")).monospace(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{avg:.3}")).monospace().weak(),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{:.0}%", rate * 100.0))
                                        .monospace()
                                        .weak(),
                                );
                                ui.end_row();
                            }
                        });
                });
                // Clicking a row selects the node, so the monitor is a way in
                // rather than a wall of numbers to go hunting from.
                if let Some(id) = jump {
                    self.select_only(id);
                }
            });
        self.show_monitor = open;
    }

    /// The timeline strip: a scrubbable playhead and the loop range.
    ///
    /// The playhead is the same `CookContext` the whole network reads, so
    /// dragging it drags the entire patch — every time-dependent operator, the
    /// keyframe curves, the LFOs. There is no separate "timeline time" that
    /// could get out of step with what is rendering.
    fn timeline(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .exact_size(52.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let icon = if self.playing { "⏸" } else { "▶" };
                    if ui
                        .button(icon)
                        .on_hover_text("Play / pause (Space)")
                        .clicked()
                    {
                        self.playing = !self.playing;
                    }
                    if ui
                        .button("⏮")
                        .on_hover_text("Back to the loop start")
                        .clicked()
                    {
                        self.time.time = self.loop_range.0;
                        self.time.frame = (self.loop_range.0 * self.time.fps).round() as i64;
                    }
                    ui.toggle_value(&mut self.looping, "Loop");

                    ui.label("from");
                    ui.add(
                        egui::DragValue::new(&mut self.loop_range.0)
                            .speed(0.05)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    ui.label("to");
                    ui.add(
                        egui::DragValue::new(&mut self.loop_range.1)
                            .speed(0.05)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    // An inverted range would make the wrap arithmetic
                    // meaningless; keep the end after the start rather than
                    // guarding at every use.
                    if self.loop_range.1 <= self.loop_range.0 {
                        self.loop_range.1 = self.loop_range.0 + 0.1;
                    }

                    ui.label(
                        egui::RichText::new(format!(
                            "frame {}    {:.2}s",
                            self.time.frame, self.time.time
                        ))
                        .monospace(),
                    );
                });
                self.playhead(ui);
            });
    }

    /// The scrub bar. Click or drag anywhere on it to move time.
    fn playhead(&mut self, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), 16.0),
            egui::Sense::click_and_drag(),
        );
        let (start, end) = self.loop_range;
        let span = (end - start).max(1e-6);

        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, egui::Color32::from_gray(30));

        // Second ticks, as long as they are not so dense as to be a smear.
        let seconds = span.ceil() as i64;
        if seconds <= 120 {
            for i in 0..=seconds {
                let t = start.floor() + i as f64;
                if t < start || t > end {
                    continue;
                }
                let x = rect.left() + ((t - start) / span) as f32 * rect.width();
                painter.line_segment(
                    [
                        egui::pos2(x, rect.bottom() - 5.0),
                        egui::pos2(x, rect.bottom()),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(70)),
                );
            }
        }

        if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let u = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64;
                self.time.time = start + u * span;
                self.time.frame = (self.time.time * self.time.fps).round() as i64;
                // Scrubbing owns time for this frame: letting the transport
                // also advance would fight the drag and jitter the playhead.
                self.scrubbing = true;
            }
        }

        let u = ((self.time.time - start) / span).clamp(0.0, 1.0) as f32;
        let x = rect.left() + u * rect.width();
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(230, 170, 90)),
        );
    }

    fn perform_preview_hint(&self, ui: &mut egui::Ui) {
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

    fn perform_edit_banner(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("perform_edit_banner").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Perform UI edit")
                        .strong()
                        .color(egui::Color32::from_rgb(220, 200, 140)),
                );
                if ui.button("Preview (F1)").clicked() {
                    self.perform_edit = false;
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
    fn perform_view(&mut self, ui: &mut egui::Ui) {
        let perform_comp = self.perform_ui();
        let widgets = perform_comp
            .map(|c| otd_engine::panel::panel_display_widgets(&self.graph, c))
            .unwrap_or_default();
        let mut panels: Vec<(NodeId, f64)> = Vec::new();
        self.visible.clear();

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
                                draw_panel_comp_look(&self.graph, comp, ui.painter(), lb);
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

    /// The output window: a second OS window showing only the viewer TOP,
    /// which is what gets dragged onto a projector (PLAN.md Phase 1).
    fn output_viewport(&mut self, ctx: &egui::Context) {
        if self.output_closed.swap(false, Ordering::Relaxed) {
            self.output_window = false;
        }
        if !self.output_window {
            return;
        }
        let tex = self.perform_output_thumbnail();
        let closed = self.output_closed.clone();
        let builder = egui::ViewportBuilder::default()
            .with_title("OpenTouchDesigner — Output")
            .with_inner_size([1280.0, 720.0])
            .with_fullscreen(self.output_fullscreen);

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

    fn side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("params")
            .default_size(400.0)
            .resizable(true)
            .show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    crate::params::show(self, ui);
                });
            });
    }
}

/// What has to cook this frame: perform output, render/cook roots, execute
/// DATs, and every node whose viewer flag is on and on screen.
///
/// That last condition is what makes the demand-driven engine visible: pan a
/// heavy branch off the canvas and it stops costing anything. It is also what
/// makes perform mode cheap rather than merely dark — nothing is on the canvas
/// there, so `visible` is empty and the only thing left is the output chain.
pub(crate) fn cook_roots(
    graph: &Graph,
    viewer: Option<NodeId>,
    visible: &[NodeId],
    current: NodeId,
) -> Vec<NodeId> {
    let mut roots = Vec::new();
    if let Some(v) = viewer.filter(|v| graph.contains(*v)) {
        roots.push(v);
    }
    crate::canvas::comp_display::append_network_display_roots(graph, current, visible, &mut roots);
    for id in graph.walk() {
        if id == graph.root() {
            continue;
        }
        let node = graph.node(id);
        let wanted = node.flags.render
            || node.flags.cook
            || otd_engine::execute::is_execute(&node.op_type)
            || (node.flags.viewer && visible.contains(&id) && node.family != otd_core::Family::Comp);
        if wanted && !roots.contains(&id) {
            roots.push(id);
        }
    }
    roots
}

/// Bytes, in whatever unit keeps the number short.
fn bytes(n: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if n as f64 >= MB {
        format!("{:.1} MB", n as f64 / MB)
    } else {
        format!("{:.0} kB", n as f64 / 1024.0)
    }
}

pub(crate) fn draw_texture_stretch(
    ui: &egui::Ui,
    tid: TextureId,
    size: [u32; 2],
    stretch: crate::window_layout::TextureStretch,
) -> egui::Rect {
    let area = ui.max_rect();
    let avail = area.size();
    let tex = egui::Vec2::new(size[0].max(1) as f32, size[1].max(1) as f32);
    let rect = match stretch {
        crate::window_layout::TextureStretch::Stretch => area,
        crate::window_layout::TextureStretch::Native => {
            let s = (avail.x / tex.x).min(avail.y / tex.y).min(1.0);
            let w = tex.x * s;
            let h = tex.y * s;
            egui::Rect::from_center_size(area.center(), egui::vec2(w, h))
        }
        crate::window_layout::TextureStretch::Fill => {
            let dst = avail.x / avail.y.max(1.0);
            let src = tex.x / tex.y;
            let (w, h) = if src > dst {
                (avail.x, avail.x / src)
            } else {
                (avail.y * src, avail.y)
            };
            let mut r = egui::Rect::from_center_size(area.center(), egui::vec2(w, h));
            if w > avail.x + 0.5 || h > avail.y + 0.5 {
                let scale = (avail.x / w).max(avail.y / h);
                r = egui::Rect::from_center_size(
                    area.center(),
                    egui::vec2(w * scale, h * scale),
                );
            }
            r
        }
        crate::window_layout::TextureStretch::Fit => {
            let aspect = tex.x / tex.y;
            let (mut w, mut h) = (avail.x, avail.x / aspect);
            if h > avail.y {
                h = avail.y;
                w = h * aspect;
            }
            egui::Rect::from_center_size(area.center(), egui::vec2(w, h))
        }
    };
    ui.painter().image(
        tid,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    rect
}

/// Paint a texture centred and letterboxed.
///
/// Letterbox rather than stretch, in both the output window and perform mode:
/// a show output at the wrong aspect ratio is worse than black bars, and worse
/// still because it looks deliberate.
/// Returns the rect it painted into, which is the box a panel lays out
/// against — the picture, not the window, so a widget stays where it was put
/// relative to the image whatever shape the window is.
pub(crate) fn letterbox(ui: &egui::Ui, tid: TextureId, size: [u32; 2]) -> egui::Rect {
    let avail = ui.available_size();
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let (mut w, mut h) = (avail.x, avail.x / aspect);
    if h > avail.y {
        h = avail.y;
        w = h * aspect;
    }
    let rect = egui::Rect::from_center_size(ui.max_rect().center(), egui::vec2(w, h));
    ui.painter().image(
        tid,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    rect
}

/// Draw the panel widgets over `area`, returning the values a person changed.
///
/// The edits come back rather than being applied here: writing needs the graph
/// mutably, and the caller is already holding it to have drawn at all. Keeping
/// that split also means this function is only about pixels.
fn panel_widget_rect(area: egui::Rect, w: &otd_engine::panel::Widget) -> egui::Rect {
    egui::Rect::from_min_size(
        area.min + egui::vec2(w.rect[0] * area.width(), w.rect[1] * area.height()),
        egui::vec2(w.rect[2] * area.width(), w.rect[3] * area.height()),
    )
}

pub(crate) fn draw_panel_comp(
    ui: &mut egui::Ui,
    app: &mut OtdApp,
    comp: NodeId,
    area: egui::Rect,
    widgets: &[otd_engine::panel::Widget],
    interact: bool,
) -> Vec<(NodeId, f64)> {
    draw_panel_comp_look(&app.graph, comp, ui.painter(), area);
    draw_panel(ui, widgets, area, Some(app), interact)
}

fn draw_panel_comp_look(
    graph: &Graph,
    comp: NodeId,
    painter: &egui::Painter,
    area: egui::Rect,
) {
    let node = graph.node(comp);
    let ctx = otd_core::EvalContext::default();
    let f = |key: &str, default: f64| {
        node.param(key)
            .map(|p| p.eval(&ctx).as_f64())
            .unwrap_or(default)
    };
    let opacity = f("opacity", 1.0) as f32;
    let alpha = (opacity * f("bgalpha", 1.0) as f32).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let color = egui::Color32::from_rgba_unmultiplied(
        (f("bgcolorr", 0.08) * 255.0) as u8,
        (f("bgcolorg", 0.09) * 255.0) as u8,
        (f("bgcolorb", 0.11) * 255.0) as u8,
        (alpha * 255.0) as u8,
    );
    painter.rect_filled(area, 0.0, color);
}

fn draw_panel_widget_look(painter: &egui::Painter, rect: egui::Rect, w: &otd_engine::panel::Widget) {
    let alpha = (w.opacity * w.bg[3]).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let fill = egui::Color32::from_rgba_unmultiplied(
        (w.bg[0] * 255.0) as u8,
        (w.bg[1] * 255.0) as u8,
        (w.bg[2] * 255.0) as u8,
        (alpha * 255.0) as u8,
    );
    painter.rect_filled(rect, 4.0, fill);
}

pub(crate) fn draw_panel_texture(
    painter: &egui::Painter,
    rect: egui::Rect,
    tid: TextureId,
    size: [u32; 2],
) {
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let draw = if aspect > rect.width() / rect.height() {
        let h = rect.width() / aspect;
        egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), h))
    } else {
        let w = rect.height() * aspect;
        egui::Rect::from_center_size(rect.center(), egui::vec2(w, rect.height()))
    };
    painter.image(
        tid,
        draw,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

pub(crate) fn draw_panel(
    ui: &mut egui::Ui,
    widgets: &[otd_engine::panel::Widget],
    area: egui::Rect,
    mut app: Option<&mut OtdApp>,
    interact: bool,
) -> Vec<(NodeId, f64)> {
    use otd_engine::panel::Kind;
    let mut sorted: Vec<&otd_engine::panel::Widget> = widgets.iter().collect();
    sorted.sort_by(|a, b| {
        a.layer
            .partial_cmp(&b.layer)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut hits: Vec<(otd_engine::panel::Widget, egui::Rect)> = Vec::new();
    let mut changed = Vec::new();
    for w in sorted {
        if !w.display {
            continue;
        }
        let rect = panel_widget_rect(area, w);
        if !ui.max_rect().intersects(rect) {
            continue;
        }
        if matches!(w.kind, Kind::Container) {
            if let Some(app) = app.as_mut() {
                let nested =
                    otd_engine::panel::panel_display_widgets(&app.graph, w.id);
                changed.extend(draw_panel_comp(
                    ui, app, w.id, rect, &nested, interact,
                ));
            } else {
                draw_panel_widget_look(ui.painter(), rect, w);
            }
            continue;
        }
        if let Some(app) = app.as_mut() {
            if let Some(top) = crate::canvas::comp_display::comp_view_top(&app.graph, w.id) {
                if let Some((tid, size)) = app.thumbnail(top) {
                    draw_panel_texture(ui.painter(), rect, tid, size);
                }
            }
        }
        if interact && w.enabled {
            hits.push(((*w).clone(), rect));
        }
    }
    if !interact {
        return changed;
    }
    for (w, rect) in hits.into_iter().rev() {
        let child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        match w.kind {
            Kind::Button => {
                let on = w.value >= 0.5;
                let r = child.interact(
                    rect,
                    child.id().with(("panel-btn", w.id)),
                    egui::Sense::click(),
                );
                if w.momentary {
                    let down = r.is_pointer_button_down_on();
                    if down != on {
                        changed.push((w.id, if down { 1.0 } else { 0.0 }));
                    }
                } else if r.clicked() {
                    changed.push((w.id, if on { 0.0 } else { 1.0 }));
                }
            }
            Kind::Slider => {
                let (lo, hi) = w.range;
                let r = child.interact(
                    rect,
                    child.id().with(("panel-slider", w.id)),
                    egui::Sense::click_and_drag(),
                );
                if r.clicked() || r.dragged() {
                    let t = if w.vertical {
                        1.0 - ((r.interact_pointer_pos().unwrap_or(rect.center()).y - rect.min.y)
                            / rect.height())
                    } else {
                        (r.interact_pointer_pos().unwrap_or(rect.center()).x - rect.min.x)
                            / rect.width()
                    };
                    let value = lo + (hi - lo) * t.clamp(0.0, 1.0) as f64;
                    changed.push((w.id, value));
                }
            }
            _ => {}
        }
    }
    changed
}

