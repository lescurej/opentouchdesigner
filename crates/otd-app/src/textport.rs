use std::collections::VecDeque;

use egui::{Color32, FontId, RichText, ScrollArea};
use crate::app::OtdApp;

const MAX_LINES: usize = 2000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level {
    Info,
    Warn,
    Error,
    Cmd,
    Out,
}

struct Line {
    frame: i64,
    level: Level,
    text: String,
}

pub struct Textport {
    lines: VecDeque<Line>,
    input: String,
    scroll_to_bottom: bool,
    pub last_cook_error: Option<String>,
    input_history: VecDeque<String>,
    history_pos: usize,
}

impl Default for Textport {
    fn default() -> Self {
        Textport {
            lines: VecDeque::new(),
            input: String::new(),
            scroll_to_bottom: true,
            last_cook_error: None,
            input_history: VecDeque::new(),
            history_pos: 0,
        }
    }
}

impl Textport {
    pub fn info(&mut self, frame: i64, text: impl Into<String>) {
        self.push(frame, Level::Info, text);
    }

    pub fn warn(&mut self, frame: i64, text: impl Into<String>) {
        self.push(frame, Level::Warn, text);
    }

    pub fn error(&mut self, frame: i64, text: impl Into<String>) {
        self.push(frame, Level::Error, text);
    }

    pub(crate) fn cmd(&mut self, frame: i64, text: impl Into<String>) {
        self.push(frame, Level::Cmd, text);
    }

    pub(crate) fn out(&mut self, frame: i64, text: impl Into<String>) {
        self.push(frame, Level::Out, text);
    }

    fn push(&mut self, frame: i64, level: Level, text: impl Into<String>) {
        self.lines.push_back(Line {
            frame,
            level,
            text: text.into(),
        });
        while self.lines.len() > MAX_LINES {
            self.lines.pop_front();
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }

    pub fn show(app: &mut OtdApp, ctx: &egui::Context) {
        if !app.show_textport {
            return;
        }
        let mut open = app.show_textport;
        const LOG_H: f32 = 240.0;

        egui::Window::new("Textport")
            .id(egui::Id::new("otd_textport"))
            .open(&mut open)
            .default_size([520.0, 320.0])
            .resizable(true)
            .default_pos([24.0, 120.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Clear").clicked() {
                        app.textport.clear();
                    }
                    ui.toggle_value(&mut app.textport.scroll_to_bottom, "Auto-scroll");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if app.engines.python_error().is_some() {
                            ui.colored_label(
                                Color32::from_rgb(230, 140, 100),
                                "Python unavailable",
                            );
                        }
                    });
                });
                ui.separator();

                ScrollArea::vertical()
                    .auto_shrink([true, true])
                    .max_height(LOG_H)
                    .stick_to_bottom(app.textport.scroll_to_bottom)
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        let font = FontId::monospace(11.0);
                        for line in &app.textport.lines {
                            let (color, prefix) = match line.level {
                                Level::Info => (Color32::from_rgb(180, 182, 190), "   "),
                                Level::Warn => (Color32::from_rgb(230, 190, 110), " ! "),
                                Level::Error => (Color32::from_rgb(230, 110, 110), " * "),
                                Level::Cmd => (Color32::from_rgb(150, 200, 255), ">>>"),
                                Level::Out => (Color32::from_rgb(130, 220, 160), " = "),
                            };
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("F{:>5} ", line.frame))
                                        .monospace()
                                        .small()
                                        .color(Color32::from_rgb(90, 92, 100)),
                                );
                                ui.label(RichText::new(prefix).monospace().color(color));
                                ui.label(
                                    RichText::new(&line.text)
                                        .font(font.clone())
                                        .color(color),
                                );
                            });
                        }
                    });

                ui.separator();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut app.textport.input)
                        .font(FontId::monospace(12.0))
                        .hint_text("Python — Enter to run")
                        .desired_width(f32::INFINITY),
                );
                let enter = ui.input(|i| {
                    i.key_pressed(egui::Key::Enter) && !i.modifiers.shift
                });
                if enter && response.lost_focus() {
                    run_console_line(app);
                    response.request_focus();
                } else if response.has_focus() {
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        app.textport.history_up();
                    } else if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        app.textport.history_down();
                    }
                }
            });
        app.show_textport = open;
    }

    pub fn history_up(&mut self) -> bool {
        if self.input_history.is_empty() {
            return false;
        }
        if self.history_pos == 0 {
            self.history_pos = self.input_history.len().saturating_sub(1);
        } else {
            self.history_pos -= 1;
        }
        if let Some(s) = self.input_history.get(self.history_pos) {
            self.input = s.clone();
        }
        true
    }

    pub fn history_down(&mut self) -> bool {
        if self.input_history.is_empty() {
            return false;
        }
        if self.history_pos + 1 >= self.input_history.len() {
            self.history_pos = self.input_history.len();
            self.input.clear();
            return true;
        }
        self.history_pos += 1;
        if let Some(s) = self.input_history.get(self.history_pos) {
            self.input = s.clone();
        }
        true
    }
}

fn run_console_line(app: &mut OtdApp) {
    let line = app.textport.input.trim().to_string();
    if line.is_empty() {
        return;
    }
    let frame = app.time.frame;
    app.textport.cmd(frame, &line);
    if app.textport.input_history.back().map(|s| s.as_str()) != Some(line.as_str()) {
        app.textport.input_history.push_back(line.clone());
        while app.textport.input_history.len() > 100 {
            app.textport.input_history.pop_front();
        }
    }
    app.textport.history_pos = app.textport.input_history.len();
    app.textport.input.clear();

    match app.engines.exec_console(&app.graph, &app.time, &line) {
        Ok(out) => {
            if !out.is_empty() {
                app.textport.out(frame, out);
            }
        }
        Err(e) => app.textport.error(frame, e),
    }
    for line in app.engines.take_script_log() {
        app.textport.info(frame, line);
    }
}
