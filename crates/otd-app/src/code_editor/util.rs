use std::sync::LazyLock;

use egui::text::{CCursor, CCursorRange};
use egui::{Id, widgets::text_edit::TextEditState};
use egui_extras::syntax_highlighting::SyntectSettings;

pub(crate) static SYNTECT: LazyLock<SyntectSettings> = LazyLock::new(SyntectSettings::default);

pub(crate) fn dat_text_edit_id(owner: Id) -> Id {
    owner.with("edit")
}

pub(crate) fn syntect_language(name: &str) -> &str {
    match name {
        "python" | "py" => "Python",
        "text" | "txt" => "Plain Text",
        "rust" | "rs" => "Rust",
        "javascript" | "js" => "JavaScript",
        "typescript" | "ts" => "TypeScript",
        "glsl" => "GLSL",
        "wgsl" => "WGSL",
        _ => name,
    }
}

pub(crate) fn popup_was_open(ctx: &egui::Context, popup_id: Id) -> bool {
    ctx.read_response(popup_id).is_some()
}

pub(crate) fn set_text_edit_cursor(ctx: &egui::Context, id: Id, text: &str, byte_pos: usize) {
    let byte_pos = byte_pos.min(text.len());
    if !text.is_char_boundary(byte_pos) {
        return;
    }
    let char_idx = text[..byte_pos].chars().count();
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    state
        .cursor
        .set_char_range(Some(CCursorRange::one(CCursor::new(char_idx))));
    state.store(ctx, id);
    ctx.memory_mut(|m| m.request_focus(id));
}

pub(crate) fn byte_cursor(text: &str, state: &TextEditState) -> usize {
    let ci: usize = state
        .cursor
        .char_range()
        .map(|r| r.primary.index.0)
        .unwrap_or(0);
    text.char_indices()
        .nth(ci)
        .map(|(b, _)| b)
        .unwrap_or(text.len())
}