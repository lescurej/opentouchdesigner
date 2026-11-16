use std::path::PathBuf;

use otd_core::{History, NodeClipboard};

pub struct DocumentState {
    pub history: History,
    pub project_path: Option<PathBuf>,
    pub status: String,
    pub cook_error: Option<String>,
    pub channel_expr_clipboard: String,
    pub(crate) node_clipboard: NodeClipboard,
    pub(crate) paste_count: u32,
    pub(crate) document_dirty: bool,
    pub(crate) show_close_save_prompt: bool,
    pub(crate) pending_launch: super::PendingLaunch,
}