use egui::{Color32, Pos2, Rect, Stroke};

use super::wire_curve::{bezier, dashed_bezier, dist_to_bezier};
use otd_core::NodeId;

use crate::app::OtdApp;

use super::layout::{input_port_pos, output_port_pos, port_hit_rect};
use super::palette::family_color;
use super::ports::{output_port_count, output_port_family};
use super::types::DragState;

pub(crate) fn draw_callback_dat_links(app: &OtdApp, ui: &egui::Ui, rects: &[(NodeId, Rect)]) {
    let painter = ui.painter();
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    let zoom = app.canvas.view.zoom;
    let color = family_color(otd_core::Family::Dat).gamma_multiply(0.85);
    for (owner, dat) in app.graph.callback_dat_links() {
        if super::callback_embed::callback_embedded(app, owner) {
            continue;
        }
        let Some(owner_rect) = find(owner) else {
            continue;
        };
        let Some(dat_rect) = find(dat) else { continue };
        let a = Pos2::new(owner_rect.center().x, owner_rect.bottom());
        let b = Pos2::new(dat_rect.center().x, dat_rect.top());
        let stroke = Stroke::new((1.25 * zoom).clamp(1.0, 2.0), color);
        painter.add(dashed_bezier(a, b, stroke));
    }
}

pub(crate) fn draw_channel_expression_links(app: &OtdApp, ui: &egui::Ui, rects: &[(NodeId, Rect)]) {
    let painter = ui.painter();
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    let zoom = app.canvas.view.zoom;
    let color = family_color(otd_core::Family::Chop).gamma_multiply(0.75);
    for (src, dst) in app.graph.channel_expression_links() {
        let Some(src_rect) = find(src) else { continue };
        let Some(dst_rect) = find(dst) else { continue };
        let a = src_rect.center();
        let b = dst_rect.center();
        let animated = app.engine.is_time_dependent(src);
        let stroke = Stroke::new(
            (1.25 * zoom).clamp(1.0, 2.0),
            if animated {
                color
            } else {
                color.gamma_multiply(0.65)
            },
        );
        painter.add(dashed_bezier(a, b, stroke));
    }
}

pub(crate) fn draw_wires(
    app: &OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    _origin: Pos2,
    rects: &[(NodeId, Rect)],
) {
    let hover = hovered_wire(app, ui, response, rects);
    if hover.is_some() && app.canvas.drag.is_none() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let painter = ui.painter();
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    for (dst, dst_rect) in rects {
        let node = app.graph.node(*dst);
        for (i, slot) in node.inputs.iter().enumerate() {
            let Some(src) = slot else { continue };
            let Some((a, b)) = wire_endpoints(app, *dst, i, *src, *dst_rect, &find) else {
                continue;
            };
            let animated = app.engine.is_time_dependent(*src);
            let family = family_color(node.input_families.get(i).copied().unwrap_or(node.family));
            let selected = app.canvas.selected_wire == Some((*dst, i));
            let hot = hover == Some((*dst, i));
            let color = if selected {
                Color32::WHITE
            } else if hot || animated {
                family
            } else {
                family.gamma_multiply(0.55)
            };
            let width = if selected {
                3.5 * app.canvas.view.zoom.max(0.5)
            } else if hot {
                2.75 * app.canvas.view.zoom.max(0.5)
            } else {
                2.0 * app.canvas.view.zoom.max(0.5)
            };
            painter.add(bezier(a, b, Stroke::new(width, color)));
        }
    }
}

fn wire_endpoints(
    app: &OtdApp,
    dst: NodeId,
    input: usize,
    src: NodeId,
    dst_rect: Rect,
    find: &impl Fn(NodeId) -> Option<Rect>,
) -> Option<(Pos2, Pos2)> {
    let src_rect = find(src)?;
    let node = app.graph.node(dst);
    let count = node.inputs.len();
    let zoom = app.canvas.view.zoom;
    let out_i = node.input_source_out.get(input).copied().unwrap_or(0);
    let out_n = output_port_count(&app.graph, src);
    Some((
        output_port_pos(src_rect, out_i, out_n, zoom),
        input_port_pos(dst_rect, input, count, zoom),
    ))
}

pub(crate) fn pick_wire(
    app: &OtdApp,
    pointer: Pos2,
    rects: &[(NodeId, Rect)],
) -> Option<(NodeId, usize)> {
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    let threshold = (10.0 * app.canvas.view.zoom).clamp(8.0, 18.0);
    let mut best: Option<(NodeId, usize, f32)> = None;
    for (dst, dst_rect) in rects {
        let node = app.graph.node(*dst);
        for (i, slot) in node.inputs.iter().enumerate() {
            let Some(src) = slot else { continue };
            let Some((a, b)) = wire_endpoints(app, *dst, i, *src, *dst_rect, &find) else {
                continue;
            };
            let d = dist_to_bezier(pointer, a, b);
            if d <= threshold && best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                best = Some((*dst, i, d));
            }
        }
    }
    best.map(|(d, i, _)| (d, i))
}

fn hovered_wire(
    app: &OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    rects: &[(NodeId, Rect)],
) -> Option<(NodeId, usize)> {
    if !response.hovered() || app.canvas.drag.is_some() {
        return None;
    }
    let pointer = ui.ctx().pointer_hover_pos()?;
    if super::node_at(app, pointer).is_some() {
        return None;
    }
    pick_wire(app, pointer, rects)
}

pub(crate) fn handle_wire_click(
    app: &mut OtdApp,
    ui: &egui::Ui,
    response: &egui::Response,
    rects: &[(NodeId, Rect)],
) {
    if app.canvas.drag.is_some() {
        return;
    }
    let primary = response.clicked();
    let secondary = response.secondary_clicked();
    if !primary && !secondary {
        return;
    }
    let Some(pointer) = response
        .interact_pointer_pos()
        .or_else(|| ui.ctx().pointer_interact_pos())
    else {
        return;
    };
    let Some(wire) = pick_wire(app, pointer, rects) else {
        return;
    };
    if super::node_at(app, pointer).is_some() {
        return;
    }
    crate::canvas::input::release_param_focus(ui.ctx());
    response.request_focus();
    app.clear_selection();
    app.canvas.selected_wire = Some(wire);
    app.canvas.wire_menu_at = secondary.then_some(pointer);
    app.canvas.skip_canvas_deselect_once = true;
}

pub(crate) fn show_wire_menu(app: &mut OtdApp, ui: &egui::Ui) {
    let Some(pos) = app.canvas.wire_menu_at else {
        return;
    };
    if app.canvas.selected_wire.is_none() {
        app.canvas.wire_menu_at = None;
        return;
    }
    let mut open = true;
    egui::Popup::new(
        egui::Id::new("otd_wire_menu"),
        ui.ctx().clone(),
        pos,
        ui.layer_id(),
    )
    .kind(egui::containers::PopupKind::Menu)
    .layout(egui::Layout::top_down_justified(egui::Align::Min))
    .style(egui::containers::menu::menu_style)
    .open_bool(&mut open)
    .show(|ui| {
        let locked = app
            .canvas.selected_wire
            .is_some_and(|(dst, _)| app.graph.contains(dst) && app.graph.node(dst).flags.lock);
        if ui
            .add_enabled(!locked, egui::Button::new("Disconnect"))
            .on_hover_text("Delete")
            .clicked()
        {
            app.queue_delete_selection();
            ui.close();
        }
    });
    if !open {
        app.canvas.wire_menu_at = None;
    }
}

pub(crate) fn draw_pending_wire(
    app: &OtdApp,
    ui: &egui::Ui,
    _origin: Pos2,
    rects: &[(NodeId, Rect)],
) {
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    match app.canvas.drag {
        Some(DragState::Wire { from, output }) => {
            let Some((_, src_rect)) = rects.iter().find(|(i, _)| *i == from) else {
                return;
            };
            let out_n = output_port_count(&app.graph, from);
            let color = family_color(output_port_family(&app.graph, from, output));
            let zoom = app.canvas.view.zoom;
            ui.painter().add(bezier(
                output_port_pos(*src_rect, output, out_n, zoom),
                pointer,
                Stroke::new(2.0, color),
            ));
        }
        Some(DragState::WireTo {
            dst,
            input,
            had_wire: _,
        }) => {
            let Some((_, dst_rect)) = rects.iter().find(|(i, _)| *i == dst) else {
                return;
            };
            let count = app.graph.node(dst).inputs.len();
            let accepts = app
                .graph
                .node(dst)
                .input_families
                .get(input)
                .copied()
                .unwrap_or(app.graph.node(dst).family);
            let color = family_color(accepts);
            let zoom = app.canvas.view.zoom;
            ui.painter().add(bezier(
                input_port_pos(*dst_rect, input, count, zoom),
                pointer,
                Stroke::new(2.0, color),
            ));
        }
        _ => {}
    }
}

pub(crate) fn finish_wire(app: &mut OtdApp, ui: &egui::Ui, rects: &[(NodeId, Rect)]) {
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
    let zoom = app.canvas.view.zoom;
    let hit = |center: Pos2, input: bool| port_hit_rect(center, zoom, input).contains(pointer);
    let drop_in_void = app.canvas.canvas_rect.is_positive() && app.canvas.canvas_rect.contains(pointer);

    match app.canvas.drag {
        Some(DragState::Wire { from, output }) => {
            if app.graph.node(from).flags.lock {
                app.canvas.drag = None;
                return;
            }
            for (id, rect) in rects {
                if app.graph.node(*id).flags.lock {
                    continue;
                }
                let count = app.graph.node(*id).inputs.len();
                for i in 0..count {
                    if hit(input_port_pos(*rect, i, count, zoom), true) {
                        app.queue_connect(from, output, *id, i);
                        app.canvas.drag = None;
                        return;
                    }
                }
            }
            if super::param_drop::try_assign_wire_to_node(app, from, pointer, rects) {
                app.document.history.end_gesture();
                return;
            }
            if drop_in_void {
                let world = app.canvas.view.to_world(app.canvas.canvas_rect.min, pointer);
                app.open_create_operator_dialog_for_wire(
                    world,
                    crate::app::WireDrop::FromOutput { from, output },
                );
            }
            app.canvas.drag = None;
        }
        Some(DragState::WireTo {
            dst,
            input,
            had_wire,
        }) => {
            if app.graph.node(dst).flags.lock {
                app.canvas.drag = None;
                return;
            }
            for (id, rect) in rects {
                if app.graph.node(*id).flags.lock {
                    continue;
                }
                let out_n = output_port_count(&app.graph, *id);
                for o in 0..out_n {
                    if hit(output_port_pos(*rect, o, out_n, zoom), false) {
                        app.queue_connect(*id, o, dst, input);
                        app.canvas.drag = None;
                        return;
                    }
                }
            }
            let count = app.graph.node(dst).inputs.len();
            let back_on_input = rects
                .iter()
                .find(|(id, _)| *id == dst)
                .is_some_and(|(_, rect)| hit(input_port_pos(*rect, input, count, zoom), true));
            if had_wire && (drop_in_void || back_on_input) {
                app.queue_disconnect(dst, input);
                app.canvas.selected_wire = None;
                app.canvas.drag = None;
                return;
            }
            if drop_in_void && !had_wire {
                let world = app.canvas.view.to_world(app.canvas.canvas_rect.min, pointer);
                app.open_create_operator_dialog_for_wire(
                    world,
                    crate::app::WireDrop::ToInput { dst, input },
                );
            }
            app.canvas.drag = None;
        }
        _ => {}
    }
}
