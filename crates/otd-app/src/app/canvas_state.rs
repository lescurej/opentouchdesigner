use std::collections::{BTreeSet, HashMap};

use egui::Pos2;
use otd_core::NodeId;

use crate::canvas::{CanvasView, DragState, PendingParamDrop};

pub struct CanvasState {
    pub selected: Option<NodeId>,
    pub selection: BTreeSet<NodeId>,
    pub selected_wire: Option<(NodeId, usize)>,
    pub(crate) wire_menu_at: Option<Pos2>,
    pub viewer: Option<NodeId>,
    pub current: NodeId,
    pub view: CanvasView,
    pub network_views: HashMap<NodeId, CanvasView>,
    pub canvas_rect: egui::Rect,
    pub drag: Option<DragState>,
    pub(crate) node_drag_start: Option<Vec<(NodeId, [f32; 2])>>,
    pub(crate) pending_param_drop: Option<PendingParamDrop>,
    pub open_container_views: BTreeSet<NodeId>,
    pub callback_embedded: BTreeSet<NodeId>,
    pub viewer_top_pan: HashMap<NodeId, egui::Vec2>,
    pub viewer_top_zoom: HashMap<NodeId, f32>,
    pub viewer_dat_scroll: HashMap<NodeId, egui::Vec2>,
    pub visible: Vec<NodeId>,
    pub(crate) skip_canvas_deselect_once: bool,
    pub canvas_rename: Option<(NodeId, String)>,
    pub network_view_id: Option<egui::Id>,
}