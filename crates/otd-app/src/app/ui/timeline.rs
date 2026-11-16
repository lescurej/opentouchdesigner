use super::*;

impl OtdApp {
    /// The timeline strip: a scrubbable playhead and the loop range.
    ///
    /// The playhead is the same `CookContext` the whole network reads, so
    /// dragging it drags the entire patch — every time-dependent operator, the
    /// keyframe curves, the LFOs. There is no separate "timeline time" that
    /// could get out of step with what is rendering.
    pub(super) fn timeline(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("timeline")
            .exact_size(52.0)
            .show(ui, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    let icon = if self.transport.playing { "⏸" } else { "▶" };
                    if ui
                        .button(icon)
                        .on_hover_text("Play / pause (Space)")
                        .clicked()
                    {
                        self.transport.playing = !self.transport.playing;
                    }
                    if ui
                        .button("⏮")
                        .on_hover_text("Back to the loop start")
                        .clicked()
                    {
                        self.time.time = self.transport.loop_range.0;
                        self.time.frame = (self.transport.loop_range.0 * self.time.fps).round() as i64;
                    }
                    ui.toggle_value(&mut self.transport.looping, "Loop");

                    ui.label("from");
                    ui.add(
                        egui::DragValue::new(&mut self.transport.loop_range.0)
                            .speed(0.05)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    ui.label("to");
                    ui.add(
                        egui::DragValue::new(&mut self.transport.loop_range.1)
                            .speed(0.05)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    // An inverted range would make the wrap arithmetic
                    // meaningless; keep the end after the start rather than
                    // guarding at every use.
                    if self.transport.loop_range.1 <= self.transport.loop_range.0 {
                        self.transport.loop_range.1 = self.transport.loop_range.0 + 0.1;
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
        let (start, end) = self.transport.loop_range;
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
                self.transport.scrubbing = true;
            }
        }

        let u = ((self.time.time - start) / span).clamp(0.0, 1.0) as f32;
        let x = rect.left() + u * rect.width();
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(230, 170, 90)),
        );
    }
}
