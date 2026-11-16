mod completion;
mod editor;
mod paint;
mod util;

#[derive(Clone, Copy)]
pub struct CodeEditorConfig {
    pub language: &'static str,
    pub rows: usize,
    pub id: egui::Id,
    pub interactive: bool,
}

pub struct CodeEditorExtras<'a> {
    pub issues: &'a [otd_engine::PythonLint],
    pub op_paths: &'a [String],
    pub me_path: &'a str,
    pub graph: &'a otd_core::Graph,
    pub engines: &'a otd_engine::Engines,
}

pub fn dat_editor_id(id: otd_core::NodeId) -> egui::Id {
    egui::Id::new(("code_editor", id))
}

pub fn dat_node_viewer_editor_id(id: otd_core::NodeId) -> egui::Id {
    egui::Id::new(("code_editor", "node_viewer", id))
}

pub fn is_callback_dat_text(app: &crate::app::OtdApp, id: otd_core::NodeId) -> bool {
    app.graph.node(id).callback_owner.is_some() && app.graph.node(id).op_type == otd_dat::ops::TEXT
}

pub use editor::show;