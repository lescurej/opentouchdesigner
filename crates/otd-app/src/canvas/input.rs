use egui::{Color32, Pos2, Rect, Stroke, StrokeKind};
use otd_core::NodeId;

use crate::app::OtdApp;

use super::chop::node_world_size;
use super::types::{CanvasView, DragState};

pub(crate) fn frame_all_nodes(app: &mut OtdApp) {
    let canvas = app.canvas_rect;
    if canvas.width() <= 1.0 || canvas.height() <= 1.0 {
        return;
    }
    let origin = canvas.min;
    if !app.graph.contains(app.current) {
        return;
    }
    let ids: Vec<NodeId> = app.graph.children(app.current).to_vec();
    if ids.is_empty() {
        app.view = CanvasView::default();
        return;
    }
    let mut min = egui::Vec2::new(f32::INFINITY, f32::INFINITY);
    let mut max = egui::Vec2::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
    for id in ids {
        let p = app.graph.node(id).pos;
        min.x = min.x.min(p[0]);
        min.y = min.y.min(p[1]);
        let size = node_world_size(app, id);
        max.x = max.x.max(p[0] + size.x);
        max.y = max.y.max(p[1] + size.y);
    }
    app.view = CanvasView::fit_world_bounds(origin, canvas, min, max);
}

pub(crate) fn handle_view_input(
    app: &mut OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    _rect: Rect,
) {
    let shift = ui.input(|i| i.modifiers.shift);
    if response.drag_started() && shift {
        if let Some(p) = ui.ctx().pointer_interact_pos() {
            app.drag = Some(DragState::Marquee {
                start: p,
                additive: true,
            });
        }
    }
    if response.clicked() || response.drag_started() || response.dragged() {
        response.request_focus();
    }
    if response.dragged() && app.drag.is_none() {
        app.view.pan += response.drag_delta() / app.view.zoom;
    }
}

pub(crate) fn finish_background_click(
    app: &mut OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    origin: Pos2,
) {
    if response.clicked() && app.drag.is_none() {
        if app.skip_canvas_deselect_once {
            app.skip_canvas_deselect_once = false;
        } else {
            release_param_focus(ui.ctx());
            app.clear_selection();
        }
    }
    if response.double_clicked() && app.drag.is_none() {
        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            ui.ctx()
                .memory_mut(|mem| mem.move_focus(egui::FocusDirection::None));
            let world_pos = app.view.to_world(origin, pointer);
            app.open_create_operator_dialog(world_pos);
        }
    }
}

pub(crate) fn draw_marquee(app: &OtdApp, ui: &egui::Ui) {
    let Some(DragState::Marquee { start, .. }) = app.drag else {
        return;
    };
    let Some(p) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    let r = Rect::from_two_pos(start, p);
    ui.painter().rect_stroke(
        r,
        0.0,
        Stroke::new(1.0, Color32::from_rgb(120, 200, 130)),
        StrokeKind::Outside,
    );
    ui.painter().rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(100, 200, 120, 28));
}

pub(crate) fn finish_marquee(app: &mut OtdApp, ui: &egui::Ui, node_rects: &[(NodeId, Rect)]) {
    if !ui.ctx().input(|i| i.pointer.any_released()) {
        return;
    }
    let Some(DragState::Marquee { start, additive }) = app.drag.take() else {
        return;
    };
    let Some(p) = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.ctx().pointer_latest_pos())
    else {
        return;
    };
    let sel = Rect::from_two_pos(start, p);
    if sel.width() < 3.0 && sel.height() < 3.0 {
        return;
    }
    let hits: Vec<NodeId> = node_rects
        .iter()
        .filter(|(_, rect)| sel.intersects(*rect))
        .map(|(id, _)| *id)
        .collect();
    if hits.is_empty() {
        return;
    }
    release_param_focus(ui.ctx());
    app.selected_wire = None;
    app.skip_canvas_deselect_once = true;
    if additive {
        for id in &hits {
            app.selection.insert(*id);
        }
        if let Some(last) = hits.last() {
            app.selected = Some(*last);
        }
    } else {
        app.selection.clear();
        for id in &hits {
            app.selection.insert(*id);
        }
        app.selected = hits.first().copied();
    }
}

pub(crate) fn handle_canvas_scroll(app: &mut OtdApp, ui: &egui::Ui, origin: Pos2, rect: Rect) {
    let Some(pointer) = ui.ctx().pointer_hover_pos() else {
        return;
    };
    if !rect.contains(pointer) {
        return;
    }
    if app
        .create_dialog_rect
        .is_some_and(|r| r.contains(pointer))
    {
        return;
    }
    let scroll = ui.ctx().input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() > 0.0 {
        let before = app.view.to_world(origin, pointer);
        app.view.zoom = (app.view.zoom * (1.0 + scroll * 0.002)).clamp(0.15, 3.0);
        let after = app.view.to_world(origin, pointer);
        app.view.pan += after - before;
    }
}

pub(crate) fn sample_input(ui: &egui::Ui, rect: Rect) -> otd_chop::InputState {
    let ctx = ui.ctx();
    let pos = ctx.pointer_latest_pos().unwrap_or(rect.center());
    ctx.input(|i| otd_chop::InputState {
        mouse: [
            (pos.x - rect.min.x) / rect.width().max(1.0) - 0.5,
            0.5 - (pos.y - rect.min.y) / rect.height().max(1.0),
        ],
        buttons: [
            i.pointer.primary_down(),
            i.pointer.secondary_down(),
            i.pointer.middle_down(),
        ],
        wheel: i.smooth_scroll_delta.y,
        keys: i
            .keys_down
            .iter()
            .map(|k| format!("{k:?}").to_lowercase())
            .collect(),
    })
}

/// Clears focused text fields. Must not run inside [`egui::Context::input_mut`],
/// [`egui::Context::data_mut`], or [`egui::Context::memory_mut`] closures — those
/// already hold a write lock and nesting panics in debug builds.
pub(crate) fn release_param_focus(ctx: &egui::Context) {
    ctx.memory_mut(|mem| mem.move_focus(egui::FocusDirection::None));
}

pub(crate) fn operator_clipboard_keys(app: &mut OtdApp, ctx: &egui::Context) {
    if app.perform_preview_only() {
        return;
    }
    if let Some(id) = app.selected {
        if !app.selection.contains(&id) {
            app.selection.insert(id);
        }
    }
    let text_focus = ctx.text_edit_focused();
    if text_focus {
        return;
    }
    let has_selection = !app.selection.is_empty() || app.selected.is_some();
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
        if has_selection && !text_focus && i.consume_key(cmd, egui::Key::C) {
            do_copy = true;
        }
        if has_selection && !text_focus && i.consume_key(cmd, egui::Key::X) {
            do_cut = true;
        }
        if i.consume_key(cmd, egui::Key::V) {
            do_paste = true;
        }

        i.events.retain(|ev| match ev {
            egui::Event::Copy if has_selection && !text_focus => {
                do_copy = true;
                false
            }
            egui::Event::Cut if has_selection && !text_focus => {
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
        if app.load_operator_clipboard_from_text(&text) {
            do_paste = true;
        } else if app.has_operator_clipboard() {
            do_paste = true;
        }
    }

    if do_copy && has_selection && !text_focus {
        release_param_focus(ctx);
        app.copy_selection(ctx);
        return;
    }
    if do_cut && has_selection && !text_focus {
        release_param_focus(ctx);
        app.cut_selection(ctx);
        return;
    }
    can_paste = app.has_operator_clipboard();
    if do_paste && can_paste && !text_focus {
        release_param_focus(ctx);
        let world = if app.canvas_rect.is_positive() {
            let pointer = ctx
                .pointer_latest_pos()
                .filter(|p| app.canvas_rect.contains(*p))
                .unwrap_or_else(|| app.canvas_rect.center());
            app.view
                .to_world(app.canvas_rect.min, pointer)
        } else {
            egui::vec2(120.0, 120.0)
        };
        app.queue_paste_clipboard([world.x, world.y]);
    }
}

pub(crate) fn handle_keys(app: &mut OtdApp, ui: &egui::Ui, _origin: Pos2, _rect: Rect) {
    operator_clipboard_keys(app, ui.ctx());

    let ctx = ui.ctx();
    if !ctx.text_edit_focused() {
        let mut select_all = false;
        let mut delete = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND, egui::Key::A) {
                select_all = true;
            } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Delete)
                || i.consume_key(egui::Modifiers::NONE, egui::Key::Backspace)
            {
                delete = true;
            }
        });
        if select_all {
            release_param_focus(ctx);
            app.select_all();
        } else if delete {
            release_param_focus(ctx);
            app.queue_delete_selection();
        }
    }

    if ui.ctx().input(|i| i.pointer.any_released()) {
        if let Some(DragState::Node { id, .. }) = app.drag {
            let ids = super::node::node_drag_group(app, id);
            for nid in ids {
                if app.graph.contains(nid) && !app.graph.node(nid).flags.lock {
                    let p = app.graph.node(nid).pos;
                    app.graph.node_mut_quiet(nid).pos = app.snap_world_pos(p);
                }
            }
            app.node_drag_start = None;
            app.drag = None;
            app.history.end_gesture();
        }
    }

    if !ui.ctx().text_edit_focused() {
        ui.ctx().input_mut(|i| {
            if i.consume_key(egui::Modifiers::NONE, egui::Key::H) {
                frame_all_nodes(app);
            }
        });
    }
}

pub(crate) enum SelectionFlag {
    Viewer,
    Display,
    CloneImmune,
    ViewerActive,
    Lock,
    Bypass,
}

fn selection_targets(app: &OtdApp) -> Vec<NodeId> {
    if !app.selection.is_empty() {
        app.selection
            .iter()
            .copied()
            .filter(|id| app.graph.contains(*id))
            .collect()
    } else if let Some(id) = app.selected.filter(|id| app.graph.contains(*id)) {
        vec![id]
    } else {
        Vec::new()
    }
}

pub(crate) fn toggle_flag_on_selection(app: &mut OtdApp, flag: SelectionFlag) {
    let Some(anchor) = app
        .selected
        .filter(|id| app.graph.contains(*id))
        .or_else(|| app.selection.iter().next().copied())
    else {
        return;
    };
    toggle_flag_at(app, flag, anchor);
}

pub(crate) fn toggle_flag_at(app: &mut OtdApp, flag: SelectionFlag, anchor: NodeId) {
    if !app.graph.contains(anchor) {
        return;
    }
    if matches!(flag, SelectionFlag::Display) {
        let fam = app.graph.node(anchor).family;
        if !super::flags::display_flag_allowed(fam) {
            app.status = if fam == otd_core::Family::Comp {
                "Display (D) is not used on COMP — use Viewer (V)".into()
            } else {
                "Display (D) is not used on MAT".into()
            };
            return;
        }
    }
    let ids = if app.is_selected(anchor) && app.selection.len() > 1 {
        selection_targets(app)
    } else {
        vec![anchor]
    };
    if ids.is_empty() {
        return;
    }
    let primary = if ids.contains(&anchor) {
        anchor
    } else {
        ids[0]
    };
    let node = app.graph.node(primary);
    let next = match flag {
        SelectionFlag::Viewer => !node.flags.viewer,
        SelectionFlag::Display => !node.flags.bg_display,
        SelectionFlag::CloneImmune => !node.flags.clone_immune,
        SelectionFlag::ViewerActive => !node.flags.viewer_active,
        SelectionFlag::Lock => !node.flags.lock,
        SelectionFlag::Bypass => !node.flags.bypass,
    };
    let tag = match flag {
        SelectionFlag::Viewer => "viewer flag",
        SelectionFlag::Display => "bg display",
        SelectionFlag::CloneImmune => "clone immune",
        SelectionFlag::ViewerActive => "viewer active",
        SelectionFlag::Lock => "lock",
        SelectionFlag::Bypass => "bypass",
    };
    app.edit(tag);
    let mut toggled = 0usize;
    for id in ids {
        if matches!(flag, SelectionFlag::Display)
            && !super::flags::display_flag_allowed(app.graph.node(id).family)
        {
            continue;
        }
        if app.graph.node(id).flags.lock && !matches!(flag, SelectionFlag::Lock) {
            continue;
        }
        let f = &mut app.graph.node_mut(id).flags;
        match flag {
            SelectionFlag::Viewer => f.viewer = next,
            SelectionFlag::Display => f.bg_display = next,
            SelectionFlag::CloneImmune => f.clone_immune = next,
            SelectionFlag::ViewerActive => {
                f.viewer_active = next;
                if next && !f.viewer {
                    f.viewer = true;
                }
            }
            SelectionFlag::Lock => f.lock = next,
            SelectionFlag::Bypass => f.bypass = next,
        }
        toggled += 1;
    }
    app.history.end_gesture();
    if toggled > 0 && matches!(flag, SelectionFlag::Display) {
        let state = if next { "on" } else { "off" };
        app.status = format!("Display {state} on {toggled} operator(s)");
    } else if toggled == 0 && matches!(flag, SelectionFlag::Display) {
        app.status = "Display (D) is not available on COMP or MAT".into();
    } else if toggled > 0 && matches!(flag, SelectionFlag::ViewerActive) {
        let state = if next { "on" } else { "off" };
        app.status = format!("Viewer Active {state} on {toggled} operator(s)");
    }
}
