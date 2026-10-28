use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;

use super::bodies::{draw_dat_body, draw_sop_body};
use super::chop::{chop_tile_channel_handles, draw_chop_body};
use super::flags::{
    draw_bypass_body_fx, draw_left_flag_strip, family_icon_label, handle_flag_clicks,
    node_drag_rect, NodeFlagsView,
};
use super::input::{release_param_focus, SelectionFlag, toggle_flag_at};
use super::comp_display::{comp_view_top, panel_widgets_local_to_comp};
use super::layout::{
    draw_connector_tab, input_port_pos, node_layout, output_port_pos, port_hit_rect,
};
use super::ports::{output_port_count, output_port_family};
use super::palette::family_color;
use super::types::{DragState, FOOTER_H};

pub(crate) fn draw_node(app: &mut OtdApp, ui: &mut egui::Ui, id: NodeId, rect: Rect, origin: Pos2) {
    let zoom = app.view.zoom;
    let panel_local = if app.graph.node(id).flags.viewer && app.graph.node(id).family == Family::Comp
    {
        panel_widgets_local_to_comp(&app.graph, id)
    } else {
        Vec::new()
    };
    let painter = ui.painter();
    let (
        family,
        bypassed,
        locked,
        viewer_on,
        bg_display,
        name,
        op_label,
        input_count,
        input_slots,
        input_families,
    ) = {
        let n = app.graph.node(id);
        (
            n.family,
            n.flags.bypass,
            n.flags.lock,
            n.flags.viewer,
            n.flags.bg_display,
            n.name.clone(),
            n.op_type.clone(),
            n.inputs.len(),
            n.inputs.clone(),
            n.input_families.clone(),
        )
    };
    let selected = app.is_selected(id);
    let primary = app.selected == Some(id);
    let perform_out = app.perform_ui() == Some(id) && family == Family::Comp;
    let animated = app.cook.is_time_dependent(id);
    let cook_us = app.cook.last_cook_us(id);
    let status = app.engines.node_status(&app.graph, id);
    let shader_error = status.is_some();

    let fc = family_color(family);
    let layout = node_layout(rect, zoom);
    let body = if family == Family::Chop {
        layout.body
    } else {
        layout.body.shrink2(Vec2::new(1.0, 1.0))
    };
    let footer = layout.footer;

    painter.rect_filled(rect, 0.0, Color32::from_rgb(34, 36, 42));
    let frame_w = (2.5 * zoom).clamp(1.5, 4.0) + if selected { (0.8 * zoom).clamp(0.0, 2.0) } else { 0.0 };
    let frame_c = if selected {
        if primary {
            fc
        } else {
            fc.gamma_multiply(0.82)
        }
    } else {
        fc
    };
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(frame_w, frame_c),
        StrokeKind::Outside,
    );
    painter.rect_filled(body, 0.0, Color32::from_rgb(22, 23, 26));
    if zoom > 0.3 && family != Family::Chop {
        let rail_w = 5.0 * zoom;
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(body.max.x - rail_w, body.min.y),
                body.max,
            ),
            0.0,
            Color32::from_rgb(46, 48, 54),
        );
    }
    let flags_view = NodeFlagsView {
        viewer: viewer_on,
        bg_display,
        bypass: bypassed,
        lock: locked,
    };
    draw_left_flag_strip(painter, layout.flags, fc, &flags_view, zoom);
    painter.line_segment(
        [
            Pos2::new(layout.flags.max.x, layout.flags.min.y),
            Pos2::new(layout.flags.max.x, layout.flags.max.y),
        ],
        Stroke::new(1.0, fc.gamma_multiply(0.35)),
    );

    painter.rect_filled(footer, 0.0, Color32::from_rgb(28, 30, 34));
    let name_field = Rect::from_min_max(
        Pos2::new(footer.min.x + 2.0 * zoom, footer.min.y + 2.0 * zoom),
        Pos2::new(footer.max.x - 2.0 * zoom, footer.max.y - 2.0 * zoom),
    );
    painter.rect_filled(name_field, 0.0, Color32::from_rgb(18, 19, 22));
    if zoom > 0.35 {
        painter
            .with_clip_rect(name_field.shrink2(Vec2::new(3.0 * zoom, 0.0)))
            .text(
                name_field.left_center() + Vec2::new(4.0 * zoom, 0.0),
                Align2::LEFT_CENTER,
                &name,
                FontId::monospace(10.0 * zoom),
                Color32::from_rgb(228, 230, 235),
            );
    }

    let cached_preview = viewer_on
        && !animated
        && app.cook.has_cooked(id)
        && !app.cook.cooked_this_frame(id, app.time.frame);
    if !viewer_on && zoom > 0.3 {
        let icon = family_icon_label(family, &op_label);
        for (i, line) in icon.lines().enumerate() {
            let kind = i == 0;
            painter.text(
                body.center() + Vec2::new(0.0, (i as f32 - 0.5) * 14.0 * zoom),
                Align2::CENTER_CENTER,
                line,
                FontId::proportional(if kind { 9.0 * zoom } else { 13.0 * zoom }),
                if kind {
                    Color32::from_rgb(120, 122, 132)
                } else {
                    fc.gamma_multiply(0.85)
                },
            );
        }
    }
    if viewer_on {
        match family {
        Family::Top => {
            if let Some((tid, size)) = app.thumbnail(id) {
                let aspect = size[0] as f32 / size[1].max(1) as f32;
                let draw = if aspect > body.width() / body.height() {
                    let h = body.width() / aspect;
                    Rect::from_center_size(body.center(), Vec2::new(body.width(), h))
                } else {
                    let w = body.height() * aspect;
                    Rect::from_center_size(body.center(), Vec2::new(w, body.height()))
                };
                ui.painter().image(
                    tid,
                    draw,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
        }
        // A CHOP's viewer is its waveform, for the same reason a TOP's is its
        // texture: you should be able to see what an operator is doing
        // without opening anything.
        Family::Chop => draw_chop_body(app, ui, id, body, zoom),
        Family::Dat => draw_dat_body(app, ui, id, body, zoom),
        Family::Sop => draw_sop_body(app, ui, id, body, zoom),
        Family::Comp => {
            if panel_local.is_empty()
                && !otd_engine::panel::is_panel_type(&op_label)
            {
                if let Some(out) = comp_view_top(&app.graph, id) {
                    if let Some((tid, size)) = app.thumbnail(out) {
                        let aspect = size[0] as f32 / size[1].max(1) as f32;
                        let draw = if aspect > body.width() / body.height() {
                            let h = body.width() / aspect;
                            Rect::from_center_size(body.center(), Vec2::new(body.width(), h))
                        } else {
                            let w = body.height() * aspect;
                            Rect::from_center_size(body.center(), Vec2::new(w, body.height()))
                        };
                        ui.painter().image(
                            tid,
                            draw,
                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                }
            }
        }
        _ => {}
        }
    }
    if cached_preview {
        painter.rect_filled(body, 2.0, Color32::from_black_alpha(90));
    }
    if bypassed {
        draw_bypass_body_fx(painter, body, zoom);
    }

    let painter = ui.painter();
    if zoom > 0.35 {
        if perform_out {
            painter.circle_filled(
                Pos2::new(name_field.max.x - 4.0 * zoom, footer.center().y),
                3.5 * zoom,
                Color32::from_rgb(90, 200, 110),
            );
        }
        if app.show_perf && cook_us > 0 && family != Family::Chop {
            painter.text(
                Pos2::new(footer.min.x + 4.0 * zoom, footer.min.y - 2.0 * zoom),
                Align2::LEFT_BOTTOM,
                format!("{:.2}ms", cook_us as f64 / 1000.0),
                FontId::proportional(8.0 * zoom),
                if animated {
                    Color32::from_rgb(120, 180, 120)
                } else {
                    Color32::from_rgb(90, 92, 100)
                },
            );
        }
    }

    if shader_error {
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(220, 90, 90)),
            StrokeKind::Outside,
        );
        if zoom > 0.4 {
            // The first few words of the real message, so the node itself
            // says what is wrong rather than just that something is.
            let text: String = status
                .as_deref()
                .unwrap_or("error")
                .chars()
                .take(34)
                .collect();
            painter.text(
                rect.center_bottom() - Vec2::new(0.0, FOOTER_H * zoom + 4.0),
                Align2::CENTER_BOTTOM,
                text,
                FontId::proportional(9.5 * zoom),
                Color32::from_rgb(240, 140, 140),
            );
        }
    }
    let out_count = output_port_count(&app.graph, id);
    for o in 0..out_count {
        let out_pos = output_port_pos(rect, o, out_count, zoom);
        let oc = output_port_family(&app.graph, id, o);
        draw_connector_tab(painter, out_pos, family_color(oc), zoom, false);
    }
    for (i, slot) in input_slots.iter().enumerate() {
        let p = input_port_pos(rect, i, input_count, zoom);
        let filled = slot.is_some();
        let accepts = input_families.get(i).copied().unwrap_or(family);
        let port_color = family_color(accepts);
        let color = if filled {
            port_color
        } else {
            port_color.gamma_multiply(0.55)
        };
        draw_connector_tab(painter, p, color, zoom, true);
    }

    if family == Family::Chop {
        chop_tile_channel_handles(app, ui, id, body, zoom);
    }

    let flag_click = handle_flag_clicks(app, ui, id, layout.flags, locked, zoom);

    let drag_rect = node_drag_rect(rect, layout.flags, zoom);
    let body_resp = ui.interact(drag_rect, egui::Id::new(("node", id)), Sense::click_and_drag());

    for o in 0..out_count {
        let out_pos = output_port_pos(rect, o, out_count, zoom);
        let out_rect = port_hit_rect(out_pos, zoom, false);
        let out_resp = ui.interact(
            out_rect,
            egui::Id::new(("out", id, o)),
            Sense::click_and_drag(),
        );
        if out_resp.drag_started() && !locked {
            app.drag = Some(DragState::Wire {
                from: id,
                output: o,
            });
        }
    }

    for i in 0..input_count {
        let wired = input_slots.get(i).and_then(|s| *s).is_some();
        let p = input_port_pos(rect, i, input_count, zoom);
        let r = port_hit_rect(p, zoom, true);
        let resp = ui.interact(r, egui::Id::new(("in", id, i)), Sense::click_and_drag());
        if resp.drag_started() && !locked {
            app.drag = Some(DragState::WireTo {
                dst: id,
                input: i,
                had_wire: wired,
            });
        }
        if resp.secondary_clicked() && wired && !locked {
            app.edit("unwire");
            app.history.end_gesture();
            let _ = app.graph.disconnect(id, i);
            app.selected_wire = None;
        }
    }
    if !flag_click && body_resp.clicked() {
        release_param_focus(ui.ctx());
        if ui.input(|i| i.modifiers.shift || i.modifiers.command) {
            app.select_toggle(id);
        } else {
            app.select_only(id);
        }
    }
    if body_resp.double_clicked() && family == Family::Comp {
        app.enter(id);
    }
    if !flag_click && body_resp.drag_started() && !locked {
        if ui.input(|i| i.modifiers.shift) {
            if let Some(p) = ui.ctx().pointer_interact_pos() {
                app.drag = Some(DragState::Marquee {
                    start: p,
                    additive: true,
                });
            }
        } else if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            let world = app.view.to_world(origin, pointer);
            let p = app.graph.node(id).pos;
            app.drag = Some(DragState::Node {
                id,
                grab: world - Vec2::new(p[0], p[1]),
            });
            if !app.is_selected(id) {
                app.select_only(id);
            }
        }
    }
    node_menu(app, &body_resp, id, name_field);

    if body_resp.hovered() {
        let summary = app
            .registry
            .get(&op_label)
            .map(|d| d.summary)
            .unwrap_or_default();
        body_resp.on_hover_text(format!("{op_label}\n{summary}"));
    }

    // Dragging a node moves it without dirtying it — layout is not a cook
    // input.
    if let Some(DragState::Node { id: drag_id, grab }) = app.drag {
        if drag_id == id && !app.graph.node(id).flags.lock {
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                // One entry for the whole drag: the tag names the node, so
                // every frame of it coalesces.
                app.edit(&format!("move:{id:?}"));
                let world = app.view.to_world(origin, pointer) - grab;
                // The rest of the selection travels with it, keeping the
                // shape of the layout you already arranged.
                let delta = world - Vec2::new(
                    app.graph.node(id).pos[0],
                    app.graph.node(id).pos[1],
                );
                let others: Vec<NodeId> =
                    app.selection.iter().copied().filter(|o| *o != id).collect();
                for other in others {
                    let p = app.graph.node(other).pos;
                    app.graph.node_mut_quiet(other).pos =
                        [p[0] + delta.x, p[1] + delta.y];
                }
                app.graph.node_mut_quiet(id).pos = [world.x, world.y];
            }
        }
    }
    if !panel_local.is_empty() {
        let _ = crate::app::draw_panel_comp(ui, app, id, body, &panel_local, false);
    } else if otd_engine::panel::is_panel_type(&op_label) {
        let w = otd_engine::panel::panel_display_widgets(&app.graph, id);
        if !w.is_empty() {
            let _ = crate::app::draw_panel_comp(ui, app, id, body, &w, false);
        }
    }
}

/// Right-click on a node.
///
/// Nothing here is new capability — all of it is a key, a double-click or a
/// row in the parameter panel. It exists because none of those announce
/// themselves, and a right-click is where people look first.
fn node_menu(app: &mut OtdApp, resp: &egui::Response, id: NodeId, _name_bar: Rect) {
    resp.context_menu(|ui| {
        if !app.is_selected(id) {
            app.select_only(id);
        }

        let viewer_on = app.graph.node(id).flags.viewer;
        if ui
            .selectable_label(viewer_on, "Viewer")
            .on_hover_text("Show the operator viewer on this node")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Viewer, id);
            ui.close();
        }
        let bg = app.graph.node(id).flags.bg_display;
        if ui
            .selectable_label(bg, "Display")
            .on_hover_text("Show this COMP on the network background (D) — several can be on at once")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Display, id);
            ui.close();
        }
        if app.graph.node(id).family == Family::Comp && ui.button("Enter").clicked() {
            app.enter(id);
            ui.close();
        }
        if otd_engine::panel::is_panel_type(&app.graph.node(id).op_type)
            && ui.button("View…").on_hover_text(
                "containerCOMP: floating View window (View page size). Other panels: Perform preview.",
            )
            .clicked()
        {
            app.view_panel_comp(id);
            ui.close();
        }
        if app.graph.node(id).family == Family::Comp
            && ui.button("Set Perform UI").clicked()
        {
            app.set_perform_ui(id);
            ui.close();
        }
        if app.graph.node(id).param("file").is_some() && ui.button("Replace File…").clicked() {
            crate::media::replace_file(app, id);
            ui.close();
        }

        ui.separator();
        if ui.button("Copy").on_hover_text("Cmd+C").clicked() {
            app.copy_selection(ui.ctx());
            ui.close();
        }
        if ui.button("Cut").on_hover_text("Cmd+X").clicked() {
            app.cut_selection(ui.ctx());
            ui.close();
        }
        if ui.button("Paste").on_hover_text("Cmd+V").clicked() {
            let pointer = ui.ctx().pointer_latest_pos().unwrap_or(resp.rect.center());
            let origin = app.canvas_rect.min;
            let world = app.view.to_world(origin, pointer);
            app.paste_clipboard([world.x, world.y]);
            ui.close();
        }

        ui.separator();
        let family = app.graph.node(id).family;
        ui.menu_button(format!("Add {}", family.suffix()), |ui| {
            crate::media::add_operator_menu(app, ui, Some(family));
        });

        let wired: Vec<(usize, otd_core::NodeId)> = app
            .graph
            .node(id)
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.map(|src| (i, src)))
            .collect();
        if !wired.is_empty() && !app.graph.node(id).flags.lock {
            ui.menu_button("Disconnect", |ui| {
                for (i, src) in &wired {
                    let label = format!(
                        "Input {} ← {}",
                        i,
                        app.graph.path(*src).trim_start_matches('/')
                    );
                    if ui.button(label).clicked() {
                        app.edit("unwire");
                        app.history.end_gesture();
                        let _ = app.graph.disconnect(id, *i);
                        app.selected_wire = None;
                        ui.close();
                    }
                }
            });
            ui.separator();
        }
        let bypassed = app.graph.node(id).flags.bypass;
        if ui
            .selectable_label(bypassed, "Bypass")
            .on_hover_text("Pass the input straight through — B")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Bypass, id);
            ui.close();
        }
        let locked = app.graph.node(id).flags.lock;
        if ui
            .selectable_label(locked, "Lock")
            .on_hover_text("Block edits and wiring")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Lock, id);
            ui.close();
        }
        if ui.button("Delete").clicked() {
            app.delete_selected();
            ui.close();
        }
    });
}
