use std::path::{Path, PathBuf};

use otd_ai::keys;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupMode {
    DefaultExample,
    Blank,
    CustomFile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiTheme {
    System,
    Dark,
    Light,
}

impl UiTheme {
    fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "system" => Some(Self::System),
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn to_egui(self) -> egui::ThemePreference {
        match self {
            Self::System => egui::ThemePreference::System,
            Self::Dark => egui::ThemePreference::Dark,
            Self::Light => egui::ThemePreference::Light,
        }
    }
}

impl StartupMode {
    fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "default_example" => Some(Self::DefaultExample),
            "blank" => Some(Self::Blank),
            "custom_file" => Some(Self::CustomFile),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::DefaultExample => "default_example",
            Self::Blank => "blank",
            Self::CustomFile => "custom_file",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AppSettings {
    pub startup_mode: StartupMode,
    pub startup_demo: String,
    pub startup_custom_path: PathBuf,
    pub prompt_save_on_exit: bool,
    pub stop_playing_when_minimized: bool,
    pub show_node_cook_time: bool,
    pub blur_params_on_select: bool,
    pub enable_playbar_shortcuts: bool,
    pub start_playing: bool,
    pub default_loop_start: f64,
    pub default_loop_end: f64,
    pub default_looping: bool,
    pub default_project_fps: f64,
    pub undo_limit: usize,
    pub snap_nodes_to_grid: bool,
    pub ui_theme: UiTheme,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            startup_mode: StartupMode::DefaultExample,
            startup_demo: "tunnel".into(),
            startup_custom_path: PathBuf::new(),
            prompt_save_on_exit: true,
            stop_playing_when_minimized: false,
            show_node_cook_time: true,
            blur_params_on_select: true,
            enable_playbar_shortcuts: true,
            start_playing: true,
            default_loop_start: 0.0,
            default_loop_end: 10.0,
            default_looping: false,
            default_project_fps: 60.0,
            undo_limit: otd_core::history::DEFAULT_LIMIT,
            snap_nodes_to_grid: true,
            ui_theme: UiTheme::System,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        Self::from_file(&Self::path()).unwrap_or_default()
    }

    pub fn path() -> PathBuf {
        keys::config_dir().join("settings.conf")
    }

    fn from_file(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        Some(Self::parse(&text))
    }

    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "startup_mode" => {
                    if let Some(mode) = StartupMode::parse(value) {
                        s.startup_mode = mode;
                    }
                }
                "startup_demo" => {
                    if !value.is_empty() {
                        s.startup_demo = value.to_string();
                    }
                }
                "startup_custom_path" => s.startup_custom_path = PathBuf::from(value),
                "prompt_save_on_exit" => s.prompt_save_on_exit = parse_bool(value, true),
                "stop_playing_when_minimized" => {
                    s.stop_playing_when_minimized = parse_bool(value, false);
                }
                "show_node_cook_time" => s.show_node_cook_time = parse_bool(value, true),
                "blur_params_on_select" => s.blur_params_on_select = parse_bool(value, true),
                "enable_playbar_shortcuts" => {
                    s.enable_playbar_shortcuts = parse_bool(value, true);
                }
                "start_playing" => s.start_playing = parse_bool(value, true),
                "default_loop_start" => {
                    if let Ok(v) = value.parse::<f64>() {
                        s.default_loop_start = v;
                    }
                }
                "default_loop_end" => {
                    if let Ok(v) = value.parse::<f64>() {
                        s.default_loop_end = v;
                    }
                }
                "default_looping" => s.default_looping = parse_bool(value, false),
                "default_project_fps" => {
                    if let Ok(v) = value.parse::<f64>() {
                        s.default_project_fps = v.clamp(1.0, 240.0);
                    }
                }
                "undo_limit" => {
                    if let Ok(v) = value.parse::<usize>() {
                        s.undo_limit = v;
                    }
                }
                "snap_nodes_to_grid" => s.snap_nodes_to_grid = parse_bool(value, true),
                "ui_theme" => {
                    if let Some(t) = UiTheme::parse(value) {
                        s.ui_theme = t;
                    }
                }
                _ => {}
            }
        }
        s.normalize_loop_range();
        s
    }

    fn normalize_loop_range(&mut self) {
        if self.default_loop_end <= self.default_loop_start {
            self.default_loop_end = self.default_loop_start + 0.1;
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::from(
            "# OpenTouchDesigner editor preferences.\n\
             # Not part of any project.\n",
        );
        out.push_str(&format!(
            "startup_mode = {}\n",
            self.startup_mode.as_str()
        ));
        out.push_str(&format!("startup_demo = {}\n", self.startup_demo));
        out.push_str(&format!(
            "startup_custom_path = {}\n",
            self.startup_custom_path.display()
        ));
        out.push_str(&format!(
            "prompt_save_on_exit = {}\n",
            bool_str(self.prompt_save_on_exit)
        ));
        out.push_str(&format!(
            "stop_playing_when_minimized = {}\n",
            bool_str(self.stop_playing_when_minimized)
        ));
        out.push_str(&format!(
            "show_node_cook_time = {}\n",
            bool_str(self.show_node_cook_time)
        ));
        out.push_str(&format!(
            "blur_params_on_select = {}\n",
            bool_str(self.blur_params_on_select)
        ));
        out.push_str(&format!(
            "enable_playbar_shortcuts = {}\n",
            bool_str(self.enable_playbar_shortcuts)
        ));
        out.push_str(&format!("start_playing = {}\n", bool_str(self.start_playing)));
        out.push_str(&format!(
            "default_loop_start = {}\n",
            self.default_loop_start
        ));
        out.push_str(&format!("default_loop_end = {}\n", self.default_loop_end));
        out.push_str(&format!(
            "default_looping = {}\n",
            bool_str(self.default_looping)
        ));
        out.push_str(&format!(
            "default_project_fps = {}\n",
            self.default_project_fps
        ));
        out.push_str(&format!("undo_limit = {}\n", self.undo_limit));
        out.push_str(&format!(
            "snap_nodes_to_grid = {}\n",
            bool_str(self.snap_nodes_to_grid)
        ));
        out.push_str(&format!("ui_theme = {}\n", self.ui_theme.as_str()));
        out
    }

    pub fn save(&self) -> Result<PathBuf, String> {
        let mut copy = self.clone();
        copy.normalize_loop_range();
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, copy.to_text()).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }
}

fn parse_bool(s: &str, default: bool) -> bool {
    match s {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn bool_str(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn theme_label(theme: UiTheme) -> &'static str {
    match theme {
        UiTheme::System => "System",
        UiTheme::Dark => "Dark",
        UiTheme::Light => "Light",
    }
}

pub fn show(app: &mut crate::app::OtdApp, ctx: &egui::Context) {
    if !app.show_settings {
        app.settings_draft = None;
        return;
    }
    if app.settings_draft.is_none() {
        app.settings_draft = Some(app.settings.clone());
    }
    let draft = app
        .settings_draft
        .as_mut()
        .expect("settings draft while window is open");
    let mut commit = false;
    let mut dismiss = false;
    egui::Window::new("Settings")
        .open(&mut app.show_settings)
        .default_width(520.0)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Editor preferences — saved for this user, not in the project.")
                    .weak()
                    .small(),
            );
            ui.separator();

            egui::Grid::new("otd_settings_grid")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label("Startup file mode");
                    egui::ComboBox::from_id_salt("startup_mode")
                        .selected_text(startup_mode_label(draft.startup_mode))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut draft.startup_mode,
                                StartupMode::DefaultExample,
                                startup_mode_label(StartupMode::DefaultExample),
                            );
                            ui.selectable_value(
                                &mut draft.startup_mode,
                                StartupMode::Blank,
                                startup_mode_label(StartupMode::Blank),
                            );
                            ui.selectable_value(
                                &mut draft.startup_mode,
                                StartupMode::CustomFile,
                                startup_mode_label(StartupMode::CustomFile),
                            );
                        });
                    ui.end_row();

                    if draft.startup_mode == StartupMode::DefaultExample {
                        ui.label("Default example");
                        egui::ComboBox::from_id_salt("startup_demo")
                            .selected_text(&draft.startup_demo)
                            .show_ui(ui, |ui| {
                                for name in otd_engine::demo::NAMES {
                                    ui.selectable_value(
                                        &mut draft.startup_demo,
                                        (*name).to_string(),
                                        *name,
                                    );
                                }
                            });
                        ui.end_row();
                    }

                    if draft.startup_mode == StartupMode::CustomFile {
                        ui.label("Custom startup file");
                        ui.horizontal(|ui| {
                            let mut path_text = draft.startup_custom_path.display().to_string();
                            if ui.text_edit_singleline(&mut path_text).changed() {
                                draft.startup_custom_path = PathBuf::from(path_text);
                            }
                            if ui.button("Browse…").clicked() {
                                if let Some(p) = rfd::FileDialog::new()
                                    .add_filter("OpenTouchDesigner", &["otd"])
                                    .pick_file()
                                {
                                    draft.startup_custom_path = p;
                                }
                            }
                        });
                        ui.end_row();
                    }
                });

            ui.separator();
            ui.checkbox(&mut draft.prompt_save_on_exit, "Prompt to save on exit");
            ui.checkbox(
                &mut draft.stop_playing_when_minimized,
                "Stop playing when minimized",
            );
            ui.checkbox(&mut draft.show_node_cook_time, "Show cook time on nodes");
            ui.checkbox(
                &mut draft.blur_params_on_select,
                "Release parameter focus when selecting a node",
            );
            ui.checkbox(
                &mut draft.enable_playbar_shortcuts,
                "Enable playbar shortcuts (Space)",
            );

            ui.separator();
            ui.strong("Timeline (new projects)");
            ui.checkbox(&mut draft.start_playing, "Start playing");
            ui.checkbox(&mut draft.default_looping, "Loop enabled");
            ui.horizontal(|ui| {
                ui.label("Loop start");
                ui.add(
                    egui::DragValue::new(&mut draft.default_loop_start)
                        .speed(0.1)
                        .range(-86400.0..=86400.0),
                );
                ui.label("Loop end");
                ui.add(
                    egui::DragValue::new(&mut draft.default_loop_end)
                        .speed(0.1)
                        .range(-86400.0..=86400.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Project FPS");
                ui.add(
                    egui::DragValue::new(&mut draft.default_project_fps)
                        .speed(1.0)
                        .range(1.0..=240.0),
                );
            });

            ui.separator();
            ui.strong("Network");
            ui.checkbox(&mut draft.snap_nodes_to_grid, "Snap nodes to grid");

            ui.separator();
            ui.strong("Undo");
            ui.horizontal(|ui| {
                ui.label("Undo steps (0 = off)");
                ui.add(egui::DragValue::new(&mut draft.undo_limit).speed(1).range(0..=500));
            });

            ui.separator();
            ui.strong("Appearance");
            ui.horizontal(|ui| {
                ui.label("Theme");
                egui::ComboBox::from_id_salt("ui_theme")
                    .selected_text(theme_label(draft.ui_theme))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.ui_theme, UiTheme::System, "System");
                        ui.selectable_value(&mut draft.ui_theme, UiTheme::Dark, "Dark");
                        ui.selectable_value(&mut draft.ui_theme, UiTheme::Light, "Light");
                    });
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    commit = true;
                    dismiss = true;
                }
                if ui.button("Cancel").clicked() {
                    dismiss = true;
                }
            });
        });
    if commit {
        if let Some(mut saved) = app.settings_draft.take() {
            saved.normalize_loop_range();
            app.settings = saved;
        }
        app.apply_settings(ctx);
        if let Err(e) = app.settings.save() {
            app.status = format!("Settings save failed: {e}");
        } else {
            app.status = "Settings saved".into();
        }
        app.show_settings = false;
    } else if dismiss || !app.show_settings {
        app.show_settings = false;
        app.settings_draft = None;
    }
}

fn startup_mode_label(mode: StartupMode) -> &'static str {
    match mode {
        StartupMode::DefaultExample => "Default example file",
        StartupMode::Blank => "Blank file",
        StartupMode::CustomFile => "Custom file",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let mut s = AppSettings::default();
        s.startup_mode = StartupMode::CustomFile;
        s.startup_custom_path = PathBuf::from("/tmp/show.otd");
        s.prompt_save_on_exit = false;
        s.undo_limit = 0;
        s.ui_theme = UiTheme::Light;
        let back = AppSettings::parse(&s.to_text());
        assert_eq!(back, s);
    }
}
