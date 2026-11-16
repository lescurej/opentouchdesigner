use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use egui::Pos2;
use otd_core::NodeId;

pub struct PerformShell {
    pub perform: bool,
    pub perform_edit: bool,
    pub(crate) perform_applied: bool,
    pub recorder: Option<NodeId>,
    pub output_window: bool,
    pub output_fullscreen: bool,
    pub show_window_placement: bool,
    pub main_perform_window: Option<NodeId>,
    pub main_perform_window_path: String,
    pub main_perform_ui_path: String,
    pub start_in_perform_mode: bool,
    pub designer_custom_position: bool,
    pub designer_x: f32,
    pub designer_y: f32,
    pub designer_w: f32,
    pub designer_h: f32,
    pub(crate) designer_position_pending: bool,
    pub(crate) output_closed: Arc<AtomicBool>,
    pub(crate) window_close_flags: HashMap<NodeId, Arc<AtomicBool>>,
    pub(crate) window_open_anchors: HashMap<NodeId, Pos2>,
    pub(crate) window_was_open: HashMap<NodeId, bool>,
}