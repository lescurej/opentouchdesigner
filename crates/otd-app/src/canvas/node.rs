use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;

use super::bodies::{draw_dat_body, draw_sop_body};
use super::chop::{chop_tile_channel_handles, draw_chop_body};
use super::callback_embed::{draw_fold_control, fold_strip_rect, main_node_world_height};
use super::viewer_active::{
    apply_panel_edits, handle_dat_viewer_input, handle_top_viewer_input,
    top_viewer_screen_rect_for, viewer_operating,
};
use super::flags::{
    display_flag_allowed, draw_bypass_body_fx, draw_footer_status_flags, draw_left_flag_strip,
    family_icon_label, footer_status_rects, handle_flag_clicks, handle_footer_flag_clicks,
    node_drag_rect, NodeFlagsView,
};
use super::input::{release_param_focus, SelectionFlag, toggle_flag_at};
use super::comp_display::{comp_view_top, panel_widgets_local_to_comp};
use super::layout::{
    draw_connector_tab, input_port_pos, node_layout, output_port_pos, port_hit_rect,
};
use super::ports::{output_port_count, output_port_family};
use super::palette::family_color;
use super::types::DragState;

pub(crate) fn draw_node(app: &mut OtdApp, ui: &mut egui::Ui, id: NodeId, rect: Rect, origin: Pos2) {
    let zoom = app.view.zoom;
    let main_h_world = main_node_world_height(app, id);
    let main_h_screen = main_h_world * zoom;
    let main_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + main_h_screen));
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
        viewer_active,
        clone_immune,
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
            n.flags.viewer_active,
            n.flags.clone_immune,
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

    let fc = family_color(family);
    let viewer_operate = viewer_operating(ui, viewer_on, viewer_active);
    let layout = node_layout(main_rect, zoom);
    let body = if family == Family::Chop {
        layout.body
    } else {
        layout.body.shrink2(Vec2::new(1.0, 1.0))
    };
    let footer = layout.footer;

    let fill = if selected {
        Color32::from_rgb(42, 44, 52)
    } else {
        Color32::from_rgb(34, 36, 42)
    };
    painter.rect_filled(main_rect, 0.0, fill);
    if selected {
        let halo_pad = (3.0 * zoom).clamp(2.0, 6.0);
        let halo_w = (2.5 * zoom).clamp(2.0, 4.0);
        let halo_c = if primary {
            Color32::from_rgb(255, 248, 210)
        } else {
            Color32::from_rgb(200, 210, 235)
        };
        painter.rect_stroke(
            main_rect.expand(halo_pad),
            0.0,
            Stroke::new(halo_w, halo_c),
            StrokeKind::Outside,
        );
    }
    let frame_w = (2.5 * zoom).clamp(1.5, 4.0);
    let frame_c = if selected { fc } else { fc.gamma_multiply(0.45) };
    painter.rect_stroke(
        main_rect,
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
        clone_immune,
        bypass: bypassed,
        lock: locked,
        bg_display,
        viewer_active,
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
    let (name_field, _display_rect, _active_rect) = footer_status_rects(footer, zoom);
    painter.rect_filled(name_field, 0.0, Color32::from_rgb(18, 19, 22));
    draw_footer_status_flags(painter, footer, &flags_view, zoom, family);
    let renaming = app
        .canvas_rename
        .as_ref()
        .is_some_and(|(rid, _)| *rid == id);
    if zoom > 0.35 && !renaming {
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
                let draw = if viewer_operate {
                    top_viewer_screen_rect_for(app, id, body, aspect)
                } else if aspect > body.width() / body.height() {
                    let h = body.width() / aspect;
                    Rect::from_center_size(body.center(), Vec2::new(body.width(), h))
                } else {
                    let w = body.height() * aspect;
                    Rect::from_center_size(body.center(), Vec2::new(w, body.height()))
                };
                ui.painter()
                    .with_clip_rect(body)
                    .image(
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
        Family::Dat => {
            if viewer_on && crate::params::dat_uses_text_param_editor(app, id) {
                ui.painter().rect_filled(body, 0.0, Color32::from_rgb(22, 23, 26));
            } else {
                draw_dat_body(
                    app,
                    ui,
                    id,
                    body,
                    zoom,
                    if viewer_operate {
                        super::viewer_active::dat_viewer_scroll(app, id)
                    } else {
                        Vec2::ZERO
                    },
                );
            }
        }
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

    super::node_message::paint_node_message(app, painter, id, main_rect, zoom);
    let out_count = output_port_count(&app.graph, id);
    for o in 0..out_count {
        let out_pos = output_port_pos(main_rect, o, out_count, zoom);
        let oc = output_port_family(&app.graph, id, o);
        draw_connector_tab(painter, out_pos, family_color(oc), zoom, false);
    }
    for (i, slot) in input_slots.iter().enumerate() {
        let p = input_port_pos(main_rect, i, input_count, zoom);
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

    let flag_click = handle_flag_clicks(app, ui, id, layout.flags, locked, zoom);

    let body_sense = if viewer_operate {
        Sense::hover()
    } else {
        Sense::click_and_drag()
    };
    let drag_rect = node_drag_rect(main_rect, layout.flags, footer, zoom);
    let body_resp = ui.interact(drag_rect, egui::Id::new(("node", id)), body_sense);
    let name_resp = ui.interact(
        name_field,
        egui::Id::new(("node_name", id)),
        Sense::click_and_drag(),
    );
    let footer_click = handle_footer_flag_clicks(app, ui, id, footer, locked, zoom, family);

    for o in 0..out_count {
        let out_pos = output_port_pos(main_rect, o, out_count, zoom);
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
        let p = input_port_pos(main_rect, i, input_count, zoom);
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
            app.queue_disconnect(id, i);
            app.selected_wire = None;
        }
    }
    let (chop_blocks, cmd_over_channel) = if family == Family::Chop {
        chop_tile_channel_handles(app, ui, id, body, zoom, viewer_operate)
    } else {
        (false, false)
    };
    let select_click = |app: &mut OtdApp, ui: &egui::Ui| {
        release_param_focus(ui.ctx());
        if ui.input(|i| i.modifiers.shift || i.modifiers.command) {
            app.select_toggle(id);
        } else {
            app.select_only(id);
        }
    };
    if !flag_click && !footer_click && !chop_blocks && !cmd_over_channel {
        if name_resp.clicked() {
            select_click(app, ui);
        } else if !viewer_operate && body_resp.clicked() {
            select_click(app, ui);
        }
    }
    if name_resp.double_clicked() && !locked {
        select_click(app, ui);
        app.canvas_rename = Some((id, app.graph.node(id).name.clone()));
        ui.ctx().data_mut(|d| {
            d.insert_temp(egui::Id::new(("focus_canvas_rename", id)), ());
        });
    }
    if body_resp.double_clicked() && !viewer_operate && family == Family::Comp {
        app.queue_enter_network(id);
    }
    let drag_from_name = name_resp.drag_started() && !locked;
    let drag_from_body = !viewer_operate
        && !flag_click
        && !chop_blocks
        && !cmd_over_channel
        && body_resp.drag_started()
        && !locked;
    if drag_from_name || drag_from_body {
        if ui.input(|i| i.modifiers.shift) && drag_from_body {
            if let Some(p) = ui.ctx().pointer_interact_pos() {
                app.drag = Some(DragState::Marquee {
                    start: p,
                    additive: true,
                });
            }
        } else if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            let world = app.view.to_world(origin, pointer);
            let p = app.graph.node(id).pos;
            if !app.is_selected(id) {
                app.select_only(id);
            }
            app.node_drag_start = Some(node_drag_snapshot(app, id));
            app.drag = Some(DragState::Node {
                id,
                grab: world - Vec2::new(p[0], p[1]),
            });
        }
    }
    if viewer_operate {
        node_menu(app, &name_resp, id, name_field);
    } else {
        node_menu(app, &body_resp, id, name_field);
    }

    if app
        .canvas_rename
        .as_ref()
        .is_some_and(|(rid, _)| *rid == id)
    {
        let focus_id = egui::Id::new(("focus_canvas_rename", id));
        let auto_focus = ui.ctx().data(|d| d.get_temp::<()>(focus_id).is_some());
        if auto_focus {
            ui.ctx().data_mut(|d| d.remove_temp::<()>(focus_id));
        }
        if let Some((_, draft)) = app.canvas_rename.as_mut() {
            let edit = ui.put(
                name_field,
                egui::TextEdit::singleline(draft)
                    .id_salt(("canvas_rename_field", id))
                    .font(egui::FontId::monospace(10.0 * zoom))
                    .margin(egui::Margin::symmetric(2, 0))
                    .desired_width(name_field.width()),
            );
            if auto_focus {
                edit.request_focus();
            }
            if edit.lost_focus() && ui.ctx().input(|i| !i.pointer.primary_down()) {
                app.commit_canvas_rename();
            } else if edit.has_focus() {
                if ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
                    app.canvas_rename = None;
                } else if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                    app.commit_canvas_rename();
                }
            }
        }
    }

    if body_resp.hovered() {
        if viewer_operate {
            body_resp.on_hover_text(
                "Viewer Active — operate the viewer (TOP: drag/scroll; DAT: drag/scroll table or text; CHOP: drag channels; panels: click widgets). Drag the name bar to move.",
            );
        } else {
            let summary = app
                .registry
                .get(&op_label)
                .map(|d| d.summary)
                .unwrap_or_default();
            body_resp.on_hover_text(format!("{op_label}\n{summary}"));
        }
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
                let snapped = app.snap_world_pos([world.x, world.y]);
                let delta = Vec2::new(
                    snapped[0] - app.graph.node(id).pos[0],
                    snapped[1] - app.graph.node(id).pos[1],
                );
                let others: Vec<NodeId> = node_drag_group(app, id)
                    .into_iter()
                    .filter(|o| *o != id)
                    .collect();
                for other in others {
                    let p = app.graph.node(other).pos;
                    app.graph.node_mut_quiet(other).pos =
                        [p[0] + delta.x, p[1] + delta.y];
                }
                app.graph.node_mut_quiet(id).pos = snapped;
            }
        }
    }
    let panel_interact = viewer_operate && !locked;
    let mut panel_edits = Vec::new();
    if !panel_local.is_empty() {
        panel_edits.extend(crate::app::draw_panel_comp(
            ui, app, id, body, &panel_local, panel_interact,
        ));
    } else if otd_engine::panel::is_panel_type(&op_label) {
        let w = otd_engine::panel::panel_display_widgets(&app.graph, id);
        if !w.is_empty() {
            panel_edits.extend(crate::app::draw_panel_comp(
                ui, app, id, body, &w, panel_interact,
            ));
        }
    }
    apply_panel_edits(app, &panel_edits);
    if viewer_operate && family == Family::Top && app.thumbnail(id).is_some() {
        let aspect = app
            .thumbnail(id)
            .map(|(_, s)| s[0] as f32 / s[1].max(1) as f32)
            .unwrap_or(1.0);
        handle_top_viewer_input(app, ui, id, body, aspect);
    }
    if viewer_on
        && family == Family::Dat
        && crate::params::dat_uses_text_param_editor(app, id)
    {
        crate::params::dat_node_viewer_editor(app, ui, id, body, zoom, viewer_operate);
    } else if viewer_operate && family == Family::Dat {
        handle_dat_viewer_input(app, ui, id, body);
    }
    if super::callback_embed::node_has_callback(app, id) {
        let strip = fold_strip_rect(rect, main_h_world, zoom);
        let _ = draw_fold_control(app, ui, id, strip, zoom);
    }
}

pub(crate) fn node_drag_group(app: &OtdApp, primary: NodeId) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = if app.is_selected(primary) && app.selection.len() > 1 {
        app.selection
            .iter()
            .copied()
            .filter(|n| app.graph.contains(*n) && !app.graph.node(*n).flags.lock)
            .collect()
    } else {
        vec![primary]
    };
    if !ids.contains(&primary) {
        ids.push(primary);
    }
    append_owned_callback_dats(app, &mut ids);
    ids.sort_by_key(|n| app.graph.path(*n));
    ids
}

fn append_owned_callback_dats(app: &OtdApp, ids: &mut Vec<NodeId>) {
    let anchors: Vec<NodeId> = ids.clone();
    for anchor in anchors {
        let Some(cb) = app.graph.owned_callback_dat(anchor) else {
            continue;
        };
        if !app.graph.contains(cb) || app.graph.node(cb).flags.lock {
            continue;
        }
        if !ids.contains(&cb) {
            ids.push(cb);
        }
    }
}

fn node_drag_snapshot(app: &OtdApp, id: NodeId) -> Vec<(NodeId, [f32; 2])> {
    node_drag_group(app, id)
        .into_iter()
        .map(|n| (n, app.graph.node(n).pos))
        .collect()
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
        if display_flag_allowed(app.graph.node(id).family) {
            let bg = app.graph.node(id).flags.bg_display;
            if ui
                .selectable_label(bg, "Display")
                .on_hover_text("Include this operator in the network background mosaic (D)")
                .clicked()
            {
                toggle_flag_at(app, SelectionFlag::Display, id);
                ui.close();
            }
        }
        if app.graph.node(id).family == Family::Comp && ui.button("Enter").clicked() {
            app.queue_enter_network(id);
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
            app.queue_paste_clipboard([world.x, world.y]);
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
                        app.queue_disconnect(id, *i);
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
            .on_hover_text("Block edits and wiring — L")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::Lock, id);
            ui.close();
        }
        let immune = app.graph.node(id).flags.clone_immune;
        if ui
            .selectable_label(immune, "Clone Immune")
            .on_hover_text("Keep local changes when the clone master re-syncs")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::CloneImmune, id);
            ui.close();
        }
        let active = app.graph.node(id).flags.viewer_active;
        if ui
            .selectable_label(active, "Viewer Active")
            .on_hover_text("Operate the node viewer — A")
            .clicked()
        {
            toggle_flag_at(app, SelectionFlag::ViewerActive, id);
            ui.close();
        }
        if ui.button("Delete").clicked() {
            app.queue_delete_selection();
            ui.close();
        }
    });
}
