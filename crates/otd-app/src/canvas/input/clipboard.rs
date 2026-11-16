use crate::app::OtdApp;

use super::view::{label_text_selected, release_param_focus};

pub(crate) fn operator_clipboard_keys(app: &mut OtdApp, ctx: &egui::Context) {
    if app.perform_preview_only() {
        return;
    }
    if let Some(id) = app.canvas.selected {
        if !app.canvas.selection.contains(&id) {
            app.canvas.selection.insert(id);
        }
    }
    if !crate::shortcut_routing::network_owns_keyboard_focus(app, ctx) {
        return;
    }
    let text_focus = false;
    let label_selected = label_text_selected(ctx);
    let has_selection = !app.canvas.selection.is_empty() || app.canvas.selected.is_some();
    let mut can_paste = app.has_operator_clipboard();
    if !has_selection && !can_paste {
        return;
    }
    let mut do_copy = false;
    let mut do_cut = false;
    let mut do_paste = false;
    let mut paste_text: Option<String> = None;

    ctx.input_mut(|i| {
        let cmd = egui::Modifiers::COMMAND;
        if has_selection && !text_focus && !label_selected && i.consume_key(cmd, egui::Key::C) {
            do_copy = true;
        }
        if has_selection && !text_focus && !label_selected && i.consume_key(cmd, egui::Key::X) {
            do_cut = true;
        }
        if i.consume_key(cmd, egui::Key::V) {
            do_paste = true;
        }

        i.events.retain(|ev| match ev {
            egui::Event::Copy if has_selection && !text_focus && !label_selected => {
                do_copy = true;
                false
            }
            egui::Event::Cut if has_selection && !text_focus && !label_selected => {
                do_cut = true;
                false
            }
            egui::Event::Paste(text) => {
                paste_text = Some(text.clone());
                false
            }
            _ => true,
        });
    });

    if let Some(text) = paste_text {
        if app.load_operator_clipboard_from_text(&text) || app.has_operator_clipboard() {
            do_paste = true;
        }
    }

    if do_copy && has_selection && !text_focus && !label_selected {
        release_param_focus(ctx);
        app.copy_selection(ctx);
        return;
    }
    if do_cut && has_selection && !text_focus && !label_selected {
        release_param_focus(ctx);
        app.cut_selection(ctx);
        return;
    }
    can_paste = app.has_operator_clipboard();
    if do_paste && can_paste && !text_focus {
        release_param_focus(ctx);
        let world = if app.canvas.canvas_rect.is_positive() {
            let pointer = ctx
                .pointer_latest_pos()
                .filter(|p| app.canvas.canvas_rect.contains(*p))
                .unwrap_or_else(|| app.canvas.canvas_rect.center());
            app.canvas.view.to_world(app.canvas.canvas_rect.min, pointer)
        } else {
            egui::vec2(120.0, 120.0)
        };
        app.queue_paste_clipboard([world.x, world.y]);
    }
}
