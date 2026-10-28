use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};
use otd_core::NodeId;

use crate::app::OtdApp;

use super::layout::{input_port_pos, output_port_pos, port_hit_rect};
use super::ports::{output_port_count, output_port_family};
use super::palette::family_color;
use super::types::DragState;

pub(crate) fn draw_channel_expression_links(
    app: &OtdApp,
    ui: &egui::Ui,
    rects: &[(NodeId, Rect)],
) {
    let painter = ui.painter();
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    let zoom = app.view.zoom;
    let color = family_color(otd_core::Family::Chop).gamma_multiply(0.75);
    for (src, dst) in app.graph.channel_expression_links() {
        let Some(src_rect) = find(src) else { continue };
        let Some(dst_rect) = find(dst) else { continue };
        let a = src_rect.center();
        let b = dst_rect.center();
        let animated = app.cook.is_time_dependent(src);
        let stroke = Stroke::new(
            (1.25 * zoom).clamp(1.0, 2.0),
            if animated { color } else { color.gamma_multiply(0.65) },
        );
        painter.add(dashed_bezier(a, b, stroke));
    }
}

pub(crate) fn draw_wires(app: &OtdApp, ui: &egui::Ui, _origin: Pos2, rects: &[(NodeId, Rect)]) {
    let painter = ui.painter();
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    for (dst, dst_rect) in rects {
        let node = app.graph.node(*dst);
        for (i, slot) in node.inputs.iter().enumerate() {
            let Some(src) = slot else { continue };
            let Some((a, b)) = wire_endpoints(app, *dst, i, *src, *dst_rect, &find) else {
                continue;
            };
            let animated = app.cook.is_time_dependent(*src);
            let color = family_color(node.input_families.get(i).copied().unwrap_or(node.family));
            let color = if animated {
                color
            } else {
                color.gamma_multiply(0.55)
            };
            let selected = app.selected_wire == Some((*dst, i));
            let width = if selected {
                3.5 * app.view.zoom.max(0.5)
            } else {
                2.0 * app.view.zoom.max(0.5)
            };
            painter.add(bezier(
                a,
                b,
                Stroke::new(
                    width,
                    if selected {
                        Color32::WHITE
                    } else {
                        color
                    },
                ),
            ));
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
    let zoom = app.view.zoom;
    let out_i = node.input_source_out.get(input).copied().unwrap_or(0);
    let out_n = output_port_count(&app.graph, src);
    Some((
        output_port_pos(src_rect, out_i, out_n, zoom),
        input_port_pos(dst_rect, input, count, zoom),
    ))
}

fn dist_to_bezier(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let pts = bezier_points(a, b);
    let steps = 32;
    let mut best = f32::INFINITY;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let q = sample_cubic(pts, t);
        let d = (p - q).length_sq();
        if d < best {
            best = d;
        }
    }
    best.sqrt()
}

pub(crate) fn pick_wire(
    app: &OtdApp,
    pointer: Pos2,
    rects: &[(NodeId, Rect)],
) -> Option<(NodeId, usize)> {
    let find = |id: NodeId| rects.iter().find(|(i, _)| *i == id).map(|(_, r)| *r);
    let threshold = (10.0 * app.view.zoom).clamp(8.0, 18.0);
    let mut best: Option<(NodeId, usize, f32)> = None;
    for (dst, dst_rect) in rects {
        let node = app.graph.node(*dst);
        for (i, slot) in node.inputs.iter().enumerate() {
            let Some(src) = slot else { continue };
            let Some((a, b)) = wire_endpoints(app, *dst, i, *src, *dst_rect, &find) else {
                continue;
            };
            let d = dist_to_bezier(pointer, a, b);
            if d <= threshold {
                if best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                    best = Some((*dst, i, d));
                }
            }
        }
    }
    best.map(|(d, i, _)| (d, i))
}

pub(crate) fn handle_wire_click(app: &mut OtdApp, ui: &egui::Ui, rects: &[(NodeId, Rect)]) {
    if app.drag.is_some() {
        return;
    }
    if !ui.ctx().input(|i| i.pointer.primary_clicked()) {
        return;
    }
    let Some(pointer) = ui.ctx().pointer_interact_pos() else {
        return;
    };
    let Some(wire) = pick_wire(app, pointer, rects) else {
        return;
    };
    if super::node_at(app, pointer).is_some() {
        return;
    }
    crate::canvas::input::release_param_focus(ui.ctx());
    app.clear_selection();
    app.selected_wire = Some(wire);
}

fn bezier_points(a: Pos2, b: Pos2) -> [Pos2; 4] {
    let dx = ((b.x - a.x).abs() * 0.5).max(30.0);
    [
        a,
        a + Vec2::new(dx, 0.0),
        b - Vec2::new(dx, 0.0),
        b,
    ]
}

fn sample_cubic(p: [Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let uu = u * u;
    let tt = t * t;
    let uuu = uu * u;
    let ttt = tt * t;
    Pos2::new(
        uuu * p[0].x + 3.0 * uu * t * p[1].x + 3.0 * u * tt * p[2].x + ttt * p[3].x,
        uuu * p[0].y + 3.0 * uu * t * p[1].y + 3.0 * u * tt * p[2].y + ttt * p[3].y,
    )
}

fn bezier(a: Pos2, b: Pos2, stroke: Stroke) -> Shape {
    let pts = bezier_points(a, b);
    Shape::CubicBezier(egui::epaint::CubicBezierShape::from_points_stroke(
        pts,
        false,
        Color32::TRANSPARENT,
        stroke,
    ))
}

fn dashed_bezier(a: Pos2, b: Pos2, stroke: Stroke) -> Shape {
    let pts = bezier_points(a, b);
    let steps = 48;
    let mut shapes = Vec::new();
    let mut prev = sample_cubic(pts, 0.0);
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let cur = sample_cubic(pts, t);
        if i % 2 == 1 {
            shapes.push(Shape::line_segment([prev, cur], stroke));
        }
        prev = cur;
    }
    Shape::Vec(shapes)
}

pub(crate) fn draw_pending_wire(app: &OtdApp, ui: &egui::Ui, _origin: Pos2, rects: &[(NodeId, Rect)]) {
    let Some(pointer) = ui.ctx().pointer_latest_pos() else {
        return;
    };
    match app.drag {
        Some(DragState::Wire { from, output }) => {
            let Some((_, src_rect)) = rects.iter().find(|(i, _)| *i == from) else {
                return;
            };
            let out_n = output_port_count(&app.graph, from);
            let color = family_color(output_port_family(&app.graph, from, output));
            let zoom = app.view.zoom;
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
            let zoom = app.view.zoom;
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
    let zoom = app.view.zoom;
    let hit = |center: Pos2, input: bool| port_hit_rect(center, zoom, input).contains(pointer);
    let drop_in_void = app.canvas_rect.is_positive() && app.canvas_rect.contains(pointer);

    match app.drag {
        Some(DragState::Wire { from, output }) => {
            if app.graph.node(from).flags.lock {
                app.drag = None;
                return;
            }
            for (id, rect) in rects {
                if app.graph.node(*id).flags.lock {
                    continue;
                }
                let count = app.graph.node(*id).inputs.len();
                for i in 0..count {
                    if hit(input_port_pos(*rect, i, count, zoom), true) {
                        app.edit("wire");
                        app.history.end_gesture();
                        match app.graph.connect_from_output(from, output, *id, i) {
                            Ok(()) => app.status.clear(),
                            Err(e) => app.status = e.to_string(),
                        }
                        app.drag = None;
                        return;
                    }
                }
            }
            if drop_in_void {
                let world = app.view.to_world(app.canvas_rect.min, pointer);
                app.open_create_operator_dialog_for_wire(
                    world,
                    crate::app::WireDrop::FromOutput { from, output },
                );
            }
            app.drag = None;
        }
        Some(DragState::WireTo {
            dst,
            input,
            had_wire,
        }) => {
            if app.graph.node(dst).flags.lock {
                app.drag = None;
                return;
            }
            for (id, rect) in rects {
                if app.graph.node(*id).flags.lock {
                    continue;
                }
                let out_n = output_port_count(&app.graph, *id);
                for o in 0..out_n {
                    if hit(output_port_pos(*rect, o, out_n, zoom), false) {
                        app.edit("wire");
                        app.history.end_gesture();
                        match app.graph.connect_from_output(*id, o, dst, input) {
                            Ok(()) => app.status.clear(),
                            Err(e) => app.status = e.to_string(),
                        }
                        app.drag = None;
                        return;
                    }
                }
            }
            let count = app.graph.node(dst).inputs.len();
            let back_on_input = rects
                .iter()
                .find(|(id, _)| *id == dst)
                .is_some_and(|(_, rect)| {
                    hit(input_port_pos(*rect, input, count, zoom), true)
                });
            if had_wire && (drop_in_void || back_on_input) {
                app.edit("unwire");
                app.history.end_gesture();
                let _ = app.graph.disconnect(dst, input);
                app.selected_wire = None;
                app.drag = None;
                return;
            }
            if drop_in_void && !had_wire {
                let world = app.view.to_world(app.canvas_rect.min, pointer);
                app.open_create_operator_dialog_for_wire(
                    world,
                    crate::app::WireDrop::ToInput { dst, input },
                );
            }
            app.drag = None;
        }
        _ => {}
    }
}
