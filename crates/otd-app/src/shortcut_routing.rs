//! When keyboard focus is on a text field, a settings window, or the assistant,
//! network shortcuts must not run. Only the central panel canvas background owns
//! graph-editing keys (V/D/L/B, delete, select-all ops, zoom wheel on canvas).

use egui::{Context, Pos2};

use crate::app::OtdApp;

pub fn begin_frame(app: &mut OtdApp) {
    app.floating_ui_rects.clear();
}

pub fn note_floating_window<R>(app: &mut OtdApp, window: &Option<egui::InnerResponse<R>>) {
    if let Some(w) = window {
        app.floating_ui_rects.push(w.response.rect);
    }
}

fn pointer_over_floating_ui(app: &OtdApp, p: Pos2) -> bool {
    if app.floating_ui_rects.iter().any(|r| r.contains(p)) {
        return true;
    }
    app.create_dialog_rect.is_some_and(|r| r.contains(p))
        || app.td_import_rect.is_some_and(|r| r.contains(p))
}

/// Graph-editing shortcuts (flags, delete, op clipboard, select-all nodes, Tab→create).
pub fn network_owns_keyboard_focus(app: &OtdApp, ctx: &Context) -> bool {
    if app.perform_preview_only() {
        return false;
    }
    if ctx.text_edit_focused() {
        return false;
    }
    if app.canvas.canvas_rename.is_some() {
        return false;
    }
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return ctx
            .pointer_latest_pos()
            .is_some_and(|p| app.canvas.canvas_rect.contains(p) && !pointer_over_floating_ui(app, p));
    };
    app.canvas.network_view_id == Some(focused)
}

/// **P** — hide/show parameter panel: network or inspector, not a text field elsewhere.
pub fn inspector_shortcut_context(app: &OtdApp, ctx: &Context) -> bool {
    if app.perform_preview_only() || ctx.text_edit_focused() {
        return false;
    }
    if network_owns_keyboard_focus(app, ctx) {
        return true;
    }
    let Some(focused) = ctx.memory(|m| m.focused()) else {
        return false;
    };
    ctx.read_response(focused)
        .is_some_and(|r| app.params_panel_rect.contains(r.rect.center()))
}

/// Mouse wheel zoom/pan on the node graph.
pub fn canvas_accepts_wheel_zoom(app: &OtdApp, ctx: &Context) -> bool {
    if ctx.text_edit_focused() {
        return false;
    }
    let Some(p) = ctx.pointer_latest_pos() else {
        return false;
    };
    if !app.canvas.canvas_rect.contains(p) || pointer_over_floating_ui(app, p) {
        return false;
    }
    match ctx.memory(|m| m.focused()) {
        None => true,
        Some(focused) => app.canvas.network_view_id == Some(focused),
    }
}
