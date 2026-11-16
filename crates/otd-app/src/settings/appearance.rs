use super::AppSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UiPalette {
    #[default]
    Classic,
    Ocean,
    Paper,
    Terminal,
    Vivid,
    Ember,
    Synthwave,
    Midnight,
    Aurora,
    Ink,
}

impl UiPalette {
    pub const ALL: [Self; 10] = [
        Self::Classic,
        Self::Ocean,
        Self::Paper,
        Self::Terminal,
        Self::Vivid,
        Self::Ember,
        Self::Synthwave,
        Self::Midnight,
        Self::Aurora,
        Self::Ink,
    ];

    fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "classic" => Self::Classic,
            "ocean" => Self::Ocean,
            "paper" => Self::Paper,
            "terminal" => Self::Terminal,
            "vivid" => Self::Vivid,
            "ember" => Self::Ember,
            "synthwave" => Self::Synthwave,
            "midnight" => Self::Midnight,
            "aurora" => Self::Aurora,
            "ink" => Self::Ink,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::Ocean => "ocean",
            Self::Paper => "paper",
            Self::Terminal => "terminal",
            Self::Vivid => "vivid",
            Self::Ember => "ember",
            Self::Synthwave => "synthwave",
            Self::Midnight => "midnight",
            Self::Aurora => "aurora",
            Self::Ink => "ink",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NetworkGridStyle {
    #[default]
    Palette,
    Lines,
    Dots,
}

impl NetworkGridStyle {
    fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "palette" => Self::Palette,
            "lines" => Self::Lines,
            "dots" => Self::Dots,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Palette => "palette",
            Self::Lines => "lines",
            Self::Dots => "dots",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NetworkGridDraw {
    pub min_spacing_px: f32,
    pub dot_scale: f32,
    pub line_width: f32,
}

impl NetworkGridDraw {
    pub fn from_settings(_s: &AppSettings) -> Self {
        Self {
            min_spacing_px: 6.0,
            dot_scale: 1.0,
            line_width: 1.0,
        }
    }
}

pub const CHOP_VIEWER_MAX_SAMPLES_SCAN_DEFAULT: usize = 16_384;
pub const CHOP_VIEWER_MAX_WAVEFORM_PX_DEFAULT: usize = 512;

#[derive(Clone, Copy, Debug)]
pub struct ChopViewerLimits {
    pub max_samples_scan: usize,
    pub max_waveform_px: usize,
}

impl Default for ChopViewerLimits {
    fn default() -> Self {
        Self {
            max_samples_scan: CHOP_VIEWER_MAX_SAMPLES_SCAN_DEFAULT,
            max_waveform_px: CHOP_VIEWER_MAX_WAVEFORM_PX_DEFAULT,
        }
    }
}

impl ChopViewerLimits {
    pub fn from_settings(s: &AppSettings) -> Self {
        Self {
            max_samples_scan: s.chop_viewer_max_samples_scan,
            max_waveform_px: s.chop_viewer_max_waveform_px,
        }
    }
}

pub fn clamp_network_grid_opacity(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

pub fn clamp_network_canvas_bg_gamma(v: f32) -> f32 {
    v.clamp(0.25, 2.5)
}

pub(crate) fn parse_appearance_key(s: &mut AppSettings, key: &str, value: &str) {
    match key {
        "last_opened_path" => s.last_opened_path = std::path::PathBuf::from(value),
        "ui_palette" => {
            if let Some(p) = UiPalette::parse(value) {
                s.ui_palette = p;
            }
        }
        "dat_external_editor" => s.dat_external_editor = value.to_string(),
        "network_grid_style" => {
            if let Some(g) = NetworkGridStyle::parse(value) {
                s.network_grid_style = g;
            }
        }
        "network_grid_opacity" => {
            if let Ok(v) = value.parse::<f32>() {
                s.network_grid_opacity = clamp_network_grid_opacity(v);
            }
        }
        "network_canvas_bg_gamma" => {
            if let Ok(v) = value.parse::<f32>() {
                s.network_canvas_bg_gamma = clamp_network_canvas_bg_gamma(v);
            }
        }
        "audio_latency_ms" => {
            if let Ok(v) = value.parse::<u32>() {
                s.audio_latency_ms = v.clamp(0, 500);
            }
        }
        "default_project_sample_rate" => {
            if let Ok(v) = value.parse::<f64>() {
                s.default_project_sample_rate = v.clamp(8000.0, 192_000.0);
            }
        }
        "max_time_slice_ms" => {
            if let Ok(v) = value.parse::<f64>() {
                s.max_time_slice_ms = v.max(0.0);
            }
        }
        "chop_viewer_max_samples_scan" => {
            if let Ok(v) = value.parse::<usize>() {
                s.chop_viewer_max_samples_scan = v.clamp(256, 2_000_000);
            }
        }
        "chop_viewer_max_waveform_px" => {
            if let Ok(v) = value.parse::<usize>() {
                s.chop_viewer_max_waveform_px = v.clamp(64, 4096);
            }
        }
        _ => {}
    }
}

pub(crate) fn write_appearance_fields(s: &AppSettings, out: &mut String) {
    out.push_str(&format!(
        "last_opened_path = {}\n",
        s.last_opened_path.display()
    ));
    out.push_str(&format!("ui_palette = {}\n", s.ui_palette.as_str()));
    out.push_str(&format!(
        "dat_external_editor = {}\n",
        s.dat_external_editor.replace('\n', " ")
    ));
    out.push_str(&format!(
        "network_grid_style = {}\n",
        s.network_grid_style.as_str()
    ));
    out.push_str(&format!(
        "network_grid_opacity = {}\n",
        s.network_grid_opacity
    ));
    out.push_str(&format!(
        "network_canvas_bg_gamma = {}\n",
        s.network_canvas_bg_gamma
    ));
    out.push_str(&format!("audio_latency_ms = {}\n", s.audio_latency_ms));
    out.push_str(&format!(
        "default_project_sample_rate = {}\n",
        s.default_project_sample_rate
    ));
    out.push_str(&format!("max_time_slice_ms = {}\n", s.max_time_slice_ms));
    out.push_str(&format!(
        "chop_viewer_max_samples_scan = {}\n",
        s.chop_viewer_max_samples_scan
    ));
    out.push_str(&format!(
        "chop_viewer_max_waveform_px = {}\n",
        s.chop_viewer_max_waveform_px
    ));
}
