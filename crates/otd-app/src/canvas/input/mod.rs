mod clipboard;
mod keys;
mod selection;
mod view;

pub(crate) use clipboard::operator_clipboard_keys;
pub(crate) use keys::handle_keys;
pub(crate) use selection::{toggle_flag_at, toggle_flag_on_selection, SelectionFlag};
pub(crate) use view::*;
pub(crate) fn release_param_focus(ctx: &egui::Context) {
    view::release_param_focus(ctx)
}
pub(crate) fn sample_input(ui: &egui::Ui, rect: egui::Rect) -> otd_chop::InputState {
    view::sample_input(ui, rect)
}
