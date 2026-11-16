use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::graph::NodeFlags;
use crate::param::Param;

pub const FORMAT_ID: &str = "opentouchdesigner-project";
pub const FORMAT_VERSION: u32 = 1;

/// Rates a project may run its audio devices at.
pub const PROJECT_SAMPLE_RATES: [f64; 4] = [44_100.0, 48_000.0, 96_000.0, 192_000.0];
pub const PROJECT_SAMPLE_RATE_DEFAULT: f64 = 48_000.0;

/// Snap to one of [`PROJECT_SAMPLE_RATES`]. Anything else becomes 48 kHz.
pub fn canonical_project_sample_rate(rate: f64) -> f64 {
    PROJECT_SAMPLE_RATES
        .into_iter()
        .find(|r| (*r - rate).abs() < 1.0)
        .unwrap_or(PROJECT_SAMPLE_RATE_DEFAULT)
}

/// `44.1 kHz`, `48 kHz`, `96 kHz`, `192 kHz`.
pub fn project_sample_rate_label(rate: f64) -> String {
    let rate = canonical_project_sample_rate(rate);
    if (rate - 44_100.0).abs() < 1.0 {
        "44.1 kHz".to_string()
    } else {
        format!("{:.0} kHz", rate / 1000.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("not an OpenTouchDesigner project file")]
    NotAProject,
    #[error("project format version {0} is newer than this build understands ({FORMAT_VERSION})")]
    TooNew(u32),
    #[error("unknown operator type `{op}` at {path}")]
    UnknownOp { op: String, path: String },
    #[error("{path} refers to input `{input}`, which does not exist")]
    DanglingInput { path: String, input: String },
    #[error("bad path `{0}`")]
    BadPath(String),
    #[error("{path}: could not read component file `{file}`: {reason}")]
    External {
        path: String,
        file: String,
        reason: String,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("serialise error: {0}")]
    Serialise(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeEntry {
    pub path: String,
    pub op: String,
    #[serde(default, skip_serializing_if = "is_origin")]
    pub pos: (f32, f32),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<(f32, f32)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<String>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub params: IndexMap<String, Param>,
    #[serde(default, skip_serializing_if = "is_default_flags")]
    pub flags: NodeFlags,
    /// This component's network lives in a `.otdc` file. When set, the
    /// project stores the reference and the instance's settings; the file
    /// stores everything inside.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external: Option<String>,
    /// This component tracks another one in the same project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_owner: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docked_to: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub dock_shown: bool,
}

fn default_true() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

fn is_origin(p: &(f32, f32)) -> bool {
    p.0 == 0.0 && p.1 == 0.0
}
fn is_default_flags(f: &NodeFlags) -> bool {
    *f == NodeFlags::default()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProjectSettings {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub main_perform_window: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub main_perform_ui: String,
    #[serde(default)]
    pub start_in_perform_mode: bool,
    #[serde(default)]
    pub designer_custom_position: bool,
    #[serde(default)]
    pub designer_x: f32,
    #[serde(default)]
    pub designer_y: f32,
    #[serde(default = "default_designer_w")]
    pub designer_w: f32,
    #[serde(default = "default_designer_h")]
    pub designer_h: f32,
}

fn default_designer_w() -> f32 {
    1600.0
}

fn default_designer_h() -> f32 {
    980.0
}

/// Pan and zoom of one network. `(0, 0)` at zoom `1` is the view a new
/// network opens on, and those values are left out of the file.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct NetworkView {
    #[serde(default, skip_serializing_if = "is_zero_pair")]
    pub pan: (f32, f32),
    #[serde(default = "default_view_zoom", skip_serializing_if = "is_default_zoom")]
    pub zoom: f32,
}

impl Default for NetworkView {
    fn default() -> Self {
        Self {
            pan: (0.0, 0.0),
            zoom: default_view_zoom(),
        }
    }
}

impl NetworkView {
    pub fn is_default(&self) -> bool {
        is_zero_pair(&self.pan) && is_default_zoom(&self.zoom)
    }
}

fn default_view_zoom() -> f32 {
    1.0
}

fn is_default_zoom(zoom: &f32) -> bool {
    (*zoom - 1.0).abs() < 1e-4
}

fn is_zero_pair(pair: &(f32, f32)) -> bool {
    pair.0.abs() < 1e-4 && pair.1.abs() < 1e-4
}

/// Where the network editor was looking when the project was saved.
///
/// `network` is the component that was open. Empty is the root. `views` is
/// keyed by path, and `"/"` is the root network.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectEditor {
    #[serde(default, skip_serializing_if = "is_root_path")]
    pub network: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub views: BTreeMap<String, NetworkView>,
}

fn is_root_path(path: &String) -> bool {
    path.is_empty() || path == "/"
}

fn is_default_editor(editor: &ProjectEditor) -> bool {
    is_root_path(&editor.network) && editor.views.is_empty()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub format: String,
    pub version: u32,
    #[serde(default = "default_fps")]
    pub fps: f64,
    /// Device clock for audio in and out. Omitted at 48 kHz so older files
    /// stay valid and unchanged.
    #[serde(
        default = "default_sample_rate",
        skip_serializing_if = "is_default_sample_rate"
    )]
    pub sample_rate: f64,
    pub nodes: Vec<NodeEntry>,
    #[serde(default)]
    pub settings: ProjectSettings,
    /// Network editor camera. Omitted while it is still the default view, so
    /// a patch nobody has panned stays byte-for-byte what it was.
    #[serde(default, skip_serializing_if = "is_default_editor")]
    pub editor: ProjectEditor,
}

/// Default timeline fps for new projects. Not control CHOP sample rate (60 Hz
/// in `otd_chop`), and not display refresh.
pub const DEFAULT_PROJECT_FPS: f64 = 60.0;

/// TouchDesigner default: clip time-slice duration when the host falls behind.
pub const DEFAULT_MAX_TIME_SLICE_MS: f64 = 200.0;

fn default_fps() -> f64 {
    DEFAULT_PROJECT_FPS
}

fn default_sample_rate() -> f64 {
    PROJECT_SAMPLE_RATE_DEFAULT
}

fn is_default_sample_rate(rate: &f64) -> bool {
    (*rate - PROJECT_SAMPLE_RATE_DEFAULT).abs() < 1.0
}

pub fn param_matches_default(a: &Param, b: &Param) -> bool {
    a.mode == b.mode && a.value == b.value && a.expression == b.expression
}
