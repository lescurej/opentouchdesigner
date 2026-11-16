use super::*;

impl OtdApp {
    /// The performance monitor: what each node costs, and what the GPU holds.
    ///
    /// Ranked by cost *per frame* rather than by cook time. Those are not the
    /// same number in a demand-driven engine, and ranking by the wrong one
    /// sends you off optimising a node that cooks once and then sits there.
    pub(super) fn monitor(&mut self, ctx: &egui::Context) {
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
                let frame_ms = 1000.0 / self.transport.smoothed_fps.max(1e-6);
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
                let p = self.frame_perf_probe;
                ui.label(
                    egui::RichText::new(format!(
                        "cook {:.2} ms · logic {:.2} ms · ui {:.2} ms · canvas {:.2} ms · gap {:.2} ms · {} cooked, {} cached",
                        self.transport.smoothed_cook_ms,
                        p.logic_ms,
                        p.ui_ms,
                        p.canvas_ms,
                        p.event_loop_gap_ms,
                        self.last_frame_timing.cooked,
                        self.last_frame_timing.cached
                    ))
                    .weak()
                    .small(),
                );

                let (passes, resident, pooled, created) = self.engine.top_gpu_stats();
                ui.label(
                    egui::RichText::new(format!(
                        "GPU  {} in node outputs · {} pooled · {} textures created · {} passes",
                        bytes(resident),
                        bytes(pooled),
                        created,
                        passes
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
                            self.engine.cook_frame_cost_ms(id),
                            self.snapshot_avg_cook_ms(id),
                            self.snapshot_cook_rate(id),
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
}

fn bytes(n: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if n as f64 >= MB {
        format!("{:.1} MB", n as f64 / MB)
    } else {
        format!("{:.0} kB", n as f64 / 1024.0)
    }
}
