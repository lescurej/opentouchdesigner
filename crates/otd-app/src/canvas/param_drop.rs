use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Ui};
use otd_core::{Family, NodeId, Value};

use crate::app::OtdApp;

use super::types::DragState;

pub(crate) struct PendingParamDrop {
    pub target: NodeId,
    pub source: NodeId,
    pub keys: Vec<String>,
    pub screen_pos: Pos2,
}

pub(crate) fn try_assign_wire_to_node(
    app: &mut OtdApp,
    source: NodeId,
    pointer: Pos2,
    node_rects: &[(NodeId, Rect)],
) -> bool {
    let Some((target, _)) = param_drop_target_at(app, source, pointer, node_rects) else {
        return false;
    };
    let keys = assignable_params(app, target, source);
    app.drag = None;
    if keys.len() == 1 {
        apply_param_reference(app, target, &keys[0], source);
        return true;
    }
    app.pending_param_drop = Some(PendingParamDrop {
        target,
        source,
        keys,
        screen_pos: pointer,
    });
    true
}

pub(crate) fn finish_node_drop(app: &mut OtdApp, ui: &egui::Ui, node_rects: &[(NodeId, Rect)]) {
    let Some(crate::canvas::DragState::Node { id: source, .. }) = app.drag else {
        return;
    };
    if !ui.ctx().input(|i| i.pointer.any_released()) {
        return;
    }
    let Some(pointer) = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.ctx().pointer_latest_pos())
    else {
        return;
    };
    let Some((target, _)) = param_drop_target_at(app, source, pointer, node_rects) else {
        return;
    };
    let keys = assignable_params(app, target, source);
    restore_node_drag_positions(app);
    app.drag = None;
    app.history.end_gesture();
    if keys.len() == 1 {
        apply_param_reference(app, target, &keys[0], source);
        return;
    }
    app.pending_param_drop = Some(PendingParamDrop {
        target,
        source,
        keys,
        screen_pos: pointer,
    });
}

pub(crate) fn restore_node_drag_positions(app: &mut OtdApp) {
    let Some(snap) = app.node_drag_start.take() else {
        return;
    };
    for (id, pos) in snap {
        if app.graph.contains(id) {
            app.graph.node_mut_quiet(id).pos = pos;
        }
    }
}

pub(crate) fn draw_param_drop_highlight(
    app: &OtdApp,
    ui: &mut Ui,
    node_rects: &[(NodeId, Rect)],
) {
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    if !app.canvas_rect.contains(pointer) {
        return;
    }
    let source = match app.drag {
        Some(DragState::Node { id, .. }) | Some(DragState::Wire { from: id, .. }) => id,
        _ => return,
    };
    let Some((_target, rect)) = param_drop_target_at(app, source, pointer, node_rects) else {
        return;
    };
    let zoom = app.view.zoom;
    let pad = (4.0 * zoom).clamp(2.0, 8.0);
    let r = rect.expand(pad);
    let corner = (6.0 * zoom).clamp(4.0, 10.0);
    let accent = Color32::from_rgb(100, 190, 255);
    let painter = ui.painter();
    painter.rect_filled(r, corner, Color32::from_rgba_unmultiplied(100, 190, 255, 45));
    painter.rect_stroke(
        r,
        corner,
        Stroke::new((3.0 * zoom).clamp(2.0, 5.0), accent),
        StrokeKind::Outside,
    );
}

pub(crate) fn draw_param_drop_menu(app: &mut OtdApp, ctx: &egui::Context) {
    let Some(pending) = app.pending_param_drop.take() else {
        return;
    };
    let source_name = app.graph.node(pending.source).name.clone();
    let target_name = app.graph.node(pending.target).name.clone();
    let mut open = true;
    let mut chosen: Option<String> = None;
    egui::Window::new("Assign operator")
        .collapsible(false)
        .resizable(false)
        .default_pos(pending.screen_pos)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.label(format!("Drop {source_name} onto {target_name}"));
            ui.separator();
            for key in &pending.keys {
                let label = app.graph.node(pending.target).params[key]
                    .label
                    .clone();
                let text = if label.is_empty() {
                    key.clone()
                } else {
                    format!("{label}  ({key})")
                };
                if ui.button(text).clicked() {
                    chosen = Some(key.clone());
                }
            }
        });
    if let Some(key) = chosen {
        app.queue_assign_param_op_ref(pending.target, key, pending.source);
    } else if open {
        app.pending_param_drop = Some(pending);
    }
}

fn param_drop_target_at(
    app: &OtdApp,
    source: NodeId,
    pointer: Pos2,
    node_rects: &[(NodeId, Rect)],
) -> Option<(NodeId, Rect)> {
    let (target, rect) = node_rects
        .iter()
        .find(|(id, rect)| *id != source && rect.contains(pointer))?;
    if app.graph.node(*target).flags.lock || app.graph.node(source).flags.lock {
        return None;
    }
    if assignable_params(app, *target, source).is_empty() {
        return None;
    }
    Some((*target, *rect))
}

pub(crate) fn apply_param_reference(app: &mut OtdApp, target: NodeId, key: &str, source: NodeId) {
    let Some(path) = app.graph.relative_op_path(target, source) else {
        app.status = "Could not build an operator path".into();
        return;
    };
    app.edit("assign-op");
    match app.graph.set_param(target, key, Value::Str(path.clone())) {
        Ok(()) => {
            if key == otd_dat::callbacks::PARAM_CALLBACKS {
                otd_dat::callbacks::link_callbacks_dat(&mut app.graph, target, source);
                otd_dat::callbacks::sync_owner_callback_source(&mut app.graph, target);
            }
            app.select_only(target);
            app.status = format!("Set {key} → {path}");
        }
        Err(e) => app.status = e.to_string(),
    }
    app.history.end_gesture();
}

fn assignable_params(app: &OtdApp, target: NodeId, source: NodeId) -> Vec<String> {
    let op_type = app.graph.node(target).op_type.clone();
    let mut keys: Vec<String> = app
        .graph
        .node(target)
        .params
        .iter()
        .filter(|(key, p)| is_op_reference_param(&op_type, key.as_str(), p.is_path_ref()))
        .filter(|(key, _)| param_accepts_source(&app.graph, &op_type, key.as_str(), source))
        .map(|(k, _)| k.clone())
        .collect();
    keys.sort_by(|a, b| {
        let la = app.graph.node(target).params[a].label.as_str();
        let lb = app.graph.node(target).params[b].label.as_str();
        la.cmp(lb).then_with(|| a.cmp(b))
    });
    keys
}

fn is_op_reference_param(op_type: &str, key: &str, path_ref: bool) -> bool {
    path_ref
        || (op_type == "feedbackTOP" && key == "target")
        || (op_type == "selectTOP" && key == "top")
}

fn param_accepts_source(
    graph: &otd_core::Graph,
    op_type: &str,
    key: &str,
    source: NodeId,
) -> bool {
    let fam = graph.node(source).family;
    let src_type = graph.node(source).op_type.as_str();
    match (op_type, key) {
        (_, "instancechop") => fam == Family::Chop,
        (_, "instancetop") => fam == Family::Top,
        ("renderTOP", "geometry") => {
            src_type == otd_gpu::scene::GEOMETRY || fam == Family::Sop
        }
        ("renderTOP", "camera") => src_type == otd_gpu::scene::CAMERA,
        ("renderTOP", "light") => src_type == otd_gpu::scene::LIGHT,
        ("geometryCOMP", "sop") => fam == Family::Sop,
        ("geometryCOMP", "material") => fam == Family::Mat,
        ("feedbackTOP", "target") | ("selectTOP", "top") => fam == Family::Top,
        (_, "timecodeop") => fam == Family::Chop || fam == Family::Dat,
        (_, "chop") => fam == Family::Chop,
        (_, "callbacks") => fam == Family::Dat,
        (_, "op") => true,
        _ => true,
    }
}
