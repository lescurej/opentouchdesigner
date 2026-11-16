mod appearance;
mod panel;

use std::path::{Path, PathBuf};

use otd_ai::keys;

pub use appearance::{
    clamp_network_canvas_bg_gamma, clamp_network_grid_opacity, ChopViewerLimits,
    NetworkGridDraw, NetworkGridStyle, UiPalette, CHOP_VIEWER_MAX_SAMPLES_SCAN_DEFAULT,
    CHOP_VIEWER_MAX_WAVEFORM_PX_DEFAULT,
};
pub use panel::show;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupMode {
    DefaultExample,
    Blank,
    CustomFile,
    LastOpened,
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
            "last_opened" => Some(Self::LastOpened),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::DefaultExample => "default_example",
            Self::Blank => "blank",
            Self::CustomFile => "custom_file",
            Self::LastOpened => "last_opened",
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
    pub last_opened_path: PathBuf,
    pub ui_palette: UiPalette,
    pub dat_external_editor: String,
    pub network_grid_style: NetworkGridStyle,
    pub network_grid_opacity: f32,
    pub network_canvas_bg_gamma: f32,
    pub audio_latency_ms: u32,
    pub default_project_sample_rate: f64,
    pub max_time_slice_ms: f64,
    pub chop_viewer_max_samples_scan: usize,
    pub chop_viewer_max_waveform_px: usize,
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
            last_opened_path: PathBuf::new(),
            ui_palette: UiPalette::default(),
            dat_external_editor: String::new(),
            network_grid_style: NetworkGridStyle::default(),
            network_grid_opacity: 1.0,
            network_canvas_bg_gamma: 1.0,
            audio_latency_ms: 0,
            default_project_sample_rate: 44_100.0,
            max_time_slice_ms: 0.0_f64,
            chop_viewer_max_samples_scan: CHOP_VIEWER_MAX_SAMPLES_SCAN_DEFAULT,
            chop_viewer_max_waveform_px: CHOP_VIEWER_MAX_WAVEFORM_PX_DEFAULT,
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
                _ => {
                    appearance::parse_appearance_key(&mut s, key, value);
                }
            }
        }
        s.normalize_loop_range();
        s
    }

    pub(crate) fn normalize_loop_range(&mut self) {
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
        appearance::write_appearance_fields(self, &mut out);
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
pub(crate) fn parse_bool(s: &str, default: bool) -> bool {
    match s {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}
pub(crate) fn bool_str(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}
pub(crate) fn theme_label(theme: UiTheme) -> &'static str {
    match theme {
        UiTheme::System => "System",
        UiTheme::Dark => "Dark",
        UiTheme::Light => "Light",
    }
}

#[cfg(test)]
#[path = "../tests/settings__tests.rs"]
mod tests;
