//! Application state and the per-frame loop.
//!
//! The loop is: advance time -> collect cook roots -> pull -> draw. Roots are
//! the perform-output operator, every node whose viewer flag is on and on
//! screen, and render/cook roots. That visibility rule is what makes the
//! demand-driven engine
//! visible to the user: pan a heavy branch off screen and it stops costing
//! anything (PLAN.md §4).

use egui::TextureId;
use otd_core::{CookContext, Graph, NodeId, OpRegistry, Value, copy_selection, paste};
use otd_engine::FrameTiming;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::engine_ui::UiEngine;

use crate::canvas::CanvasView;

pub(crate) struct NativeTopView {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub egui_id: TextureId,
    pub version: u64,
}

pub struct OtdApp {
    pub graph: Graph,
    pub registry: OpRegistry,
    pub engine: UiEngine,
    pub last_frame_timing: FrameTiming,
    pub render_state: eframe::egui_wgpu::RenderState,

    pub time: CookContext,
    pub transport: TransportState,
    pub canvas: CanvasState,
    pub perform_shell: PerformShell,
    pub document: DocumentState,

    graph_commands: Vec<graph_commands::GraphCommand>,
    pending_media_drop: Option<(Vec<PathBuf>, Option<egui::Pos2>)>,
    pub create_dialog: Option<CreateDialog>,
    /// Screen rect of the Add operator window, for blocking canvas zoom while
    /// the pointer is over it.
    pub create_dialog_rect: Option<egui::Rect>,
    pub td_import: Option<crate::td_import::TdImportDialog>,
    pub td_import_rect: Option<egui::Rect>,
    pub show_params_panel: bool,
    pub show_perf: bool,
    /// The performance monitor window.
    pub show_monitor: bool,
    pub show_textport: bool,
    pub textport: crate::textport::Textport,
    pub code_lint: crate::code_lint::CodeLintCache,

    /// Mouse and keyboard, sampled each frame for the input CHOPs.
    pub input_state: otd_chop::InputState,
    /// Draft state for the "add a component parameter" row.
    pub new_param_name: String,
    pub new_param_type: String,
    /// Active inspector tab per node (TouchDesigner-style pages).
    pub param_inspector_page: HashMap<NodeId, String>,

    thumbs: HashMap<NodeId, NativeTopView>,
    thumb_sizes: HashMap<NodeId, [u32; 2]>,
    pending_texture_sync: Vec<NodeId>,
    ui_snapshot: engine_snapshot::EngineUiSnapshot,
    /// The assistant panel: describe a patch, get operators.
    pub assistant: crate::assistant::Assistant,

    pub settings: crate::settings::AppSettings,
    pub show_settings: bool,
    pub settings_draft: Option<crate::settings::AppSettings>,
    pub floating_ui_rects: Vec<egui::Rect>,
    pub params_panel_rect: egui::Rect,
    pub external_dat_edits: HashMap<NodeId, crate::dat_external::ExternalDatWatch>,
    pub(crate) file_dialogs: file_dialog::FileDialogState,
    pub monitors: Vec<crate::monitors::MonitorLayout>,
    blur_params_on_select: bool,
    background_engine: minimized_cook::BackgroundEngine,
    playback_activity: playback_activity::PlaybackActivity,
    frame_perf_probe: FramePerfProbe,
    last_logic_at: std::time::Instant,
}

pub(crate) enum PendingLaunch {
    None,
    Startup,
    Open(PathBuf),
}

#[derive(Default, Clone, Copy)]
pub(crate) struct FramePerfProbe {
    pub logic_ms: f64,
    pub poll_async_ms: f64,
    pub cook_wall_ms: f64,
    pub cook_ms: f64,
    pub snapshot_ms: f64,
    pub sync_ms: f64,
    pub sync_blits: u32,
    pub canvas_ms: f64,
    pub ui_ms: f64,
    pub did_cook: bool,
    pub event_loop_gap_ms: f64,
}

#[derive(Clone, Copy, Debug)]
pub enum WireDrop {
    FromOutput { from: NodeId, output: usize },
    ToInput { dst: NodeId, input: usize },
}

#[derive(Clone, Debug)]
pub struct CreateDialog {
    pub filter: String,
    pub family: otd_core::Family,
    pub world_pos: egui::Vec2,
    pub focus: bool,
    pub connect_from: Option<NodeId>,
    pub wire_drop: Option<WireDrop>,
}

mod cook;
mod engine_snapshot;
mod transport;
mod canvas_state;
mod perform_shell;
mod document;
pub(crate) mod file_dialog;
mod graph_commands;
mod minimized_cook;
mod playback_activity;
mod ui;
mod r#impl;

pub use canvas_state::CanvasState;
pub use document::DocumentState;
pub use perform_shell::PerformShell;
pub use transport::TransportState;

pub(crate) use ui::{draw_panel_comp, draw_texture_stretch};

impl Drop for OtdApp {
    fn drop(&mut self) {
        self.shutdown_background_engine();
    }
}




fn network_path(graph: &Graph, id: NodeId) -> Option<String> {
    if !graph.contains(id) {
        return None;
    }
    if id == graph.root() {
        return Some("/".to_string());
    }
    if graph.node(id).family == otd_core::Family::Comp {
        return Some(graph.path(id));
    }
    None
}

fn network_from_canvas(view: CanvasView) -> otd_core::NetworkView {
    otd_core::NetworkView {
        pan: (
            if view.pan.x.is_finite() {
                view.pan.x
            } else {
                0.0
            },
            if view.pan.y.is_finite() {
                view.pan.y
            } else {
                0.0
            },
        ),
        zoom: if view.zoom.is_finite() {
            view.zoom
        } else {
            1.0
        },
    }
}

fn canvas_from_network(view: otd_core::NetworkView) -> CanvasView {
    CanvasView {
        pan: egui::vec2(
            if view.pan.0.is_finite() {
                view.pan.0
            } else {
                0.0
            },
            if view.pan.1.is_finite() {
                view.pan.1
            } else {
                0.0
            },
        ),
        zoom: if view.zoom.is_finite() {
            view.zoom.clamp(0.15, 3.0)
        } else {
            1.0
        },
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn capture_editor_camera(
    graph: &Graph,
    current: NodeId,
    view: CanvasView,
    stored: &HashMap<NodeId, CanvasView>,
) -> otd_core::ProjectEditor {
    let mut editor = otd_core::ProjectEditor::default();
    if let Some(path) = network_path(graph, current) {
        if path != "/" {
            editor.network = path;
        }
    }
    let mut put = |id: NodeId, view: CanvasView| {
        let Some(path) = network_path(graph, id) else {
            return;
        };
        let stored = network_from_canvas(view);
        if stored.is_default() {
            return;
        }
        editor.views.insert(path, stored);
    };
    for (id, stored_view) in stored {
        if *id == current {
            continue;
        }
        put(*id, *stored_view);
    }
    put(current, view);
    editor
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn restore_editor_camera(
    graph: &Graph,
    editor: &otd_core::ProjectEditor,
) -> (NodeId, CanvasView, HashMap<NodeId, CanvasView>) {
    let mut views = HashMap::new();
    for (path, view) in &editor.views {
        let Some(id) = graph.find(path) else {
            continue;
        };
        if network_path(graph, id).is_none() {
            continue;
        }
        let canvas = canvas_from_network(*view);
        if network_from_canvas(canvas).is_default() {
            continue;
        }
        views.insert(id, canvas);
    }
    let current = if editor.network.is_empty() || editor.network == "/" {
        graph.root()
    } else {
        graph
            .find(&editor.network)
            .filter(|id| network_path(graph, *id).is_some_and(|path| path != "/"))
            .unwrap_or_else(|| graph.root())
    };
    let view = views.get(&current).copied().unwrap_or_default();
    (current, view, views)
}

#[cfg(test)]
#[path = "../tests/app_mod__tests.rs"]
mod tests;
