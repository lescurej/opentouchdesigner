use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;
use crate::canvas::callback_embed::{draw_fold_control, fold_strip_rect};
use crate::canvas::chop::chop_tile_channel_handles;
use crate::canvas::flags::{handle_flag_clicks, handle_footer_flag_clicks, node_drag_rect};
use crate::canvas::input::release_param_focus;
use crate::canvas::types::DragState;
use crate::canvas::viewer_active::{
    apply_panel_edits, handle_dat_viewer_input, handle_top_viewer_input,
};
use super::super::drag::{node_drag_group, node_drag_snapshot};
use super::super::menu::node_menu;

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_node_interaction(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: NodeId,
    rect: Rect,
    origin: Pos2,
    main_rect: Rect,
    body: Rect,
    footer: Rect,
    name_field: Rect,
    layout_flags: Rect,
    zoom: f32,
    locked: bool,
    family: Family,
    op_label: &str,
    viewer_on: bool,
    viewer_operate: bool,
    main_h_world: f32,
    panel_local: &[otd_engine::panel::Widget],
) {
    let flag_click = handle_flag_clicks(app, ui, id, layout_flags, locked, zoom);
    let body_sense = if viewer_operate {
        Sense::hover()
    } else {
        Sense::click_and_drag()
    };
    let drag_rect = node_drag_rect(main_rect, layout_flags, footer, zoom);
    let body_resp = ui.interact(drag_rect, egui::Id::new(("node", id)), body_sense);
    let name_resp = ui.interact(
        name_field,
        egui::Id::new(("node_name", id)),
        Sense::click_and_drag(),
    );
    let grip = resize_grip_rect(main_rect, zoom);
    let grip_resp = if locked {
        None
    } else {
        let resp = ui
            .interact(grip, egui::Id::new(("node_resize", id)), Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeNwSe)
            .on_hover_text("Resize");
        if resp.drag_started() {
            if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                let world = app.canvas.view.to_world(origin, pointer);
                let size = crate::canvas::callback_embed::main_node_world_size(app, id);
                app.canvas.drag = Some(DragState::Resize {
                    id,
                    start_pointer: world,
                    start_size: size,
                });
            }
        }
        Some(resp)
    };
    let grip_hot = grip_resp.as_ref().is_some_and(|r| r.hovered())
        || matches!(app.canvas.drag, Some(DragState::Resize { id: rid, .. }) if rid == id);
    if !locked && zoom > 0.4 {
        paint_resize_grip(ui, grip, zoom, grip_hot);
    }
    let footer_click = handle_footer_flag_clicks(app, ui, id, footer, locked, zoom, family);
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
    if !flag_click
        && !footer_click
        && !chop_blocks
        && !cmd_over_channel
        && (name_resp.clicked() || (!viewer_operate && body_resp.clicked()))
    {
        select_click(app, ui);
    }
    if name_resp.double_clicked() && !locked {
        select_click(app, ui);
        app.canvas.canvas_rename = Some((id, app.graph.node(id).name.clone()));
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
                app.canvas.drag = Some(DragState::Marquee {
                    start: p,
                    additive: true,
                });
            }
        } else if let Some(pointer) = ui.ctx().pointer_interact_pos() {
            let world = app.canvas.view.to_world(origin, pointer);
            let p = app.graph.node(id).pos;
            if !app.is_selected(id) {
                app.select_only(id);
            }
            app.canvas.node_drag_start = Some(node_drag_snapshot(app, id));
            app.canvas.drag = Some(DragState::Node {
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
        .canvas.canvas_rename
        .as_ref()
        .is_some_and(|(rid, _)| *rid == id)
    {
        let focus_id = egui::Id::new(("focus_canvas_rename", id));
        let auto_focus = ui.ctx().data(|d| d.get_temp::<()>(focus_id).is_some());
        if auto_focus {
            ui.ctx().data_mut(|d| d.remove_temp::<()>(focus_id));
        }
        if let Some((_, draft)) = app.canvas.canvas_rename.as_mut() {
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
                    app.canvas.canvas_rename = None;
                } else if ui.ctx().input(|i| i.key_pressed(egui::Key::Enter)) {
                    app.commit_canvas_rename();
                }
            }
        }
    }
    if body_resp.hovered() || name_resp.hovered() {
        let summary = app
            .registry
            .get(op_label)
            .map(|d| d.summary.to_string())
            .unwrap_or_default();
        let mut tip = {
            let rt = app.engine_rt();
            otd_engine::node_infotip(
                &rt.engines,
                &rt.cook,
                &rt.graph,
                id,
                rt.time.frame,
                &summary,
            )
        };
        if viewer_operate {
            tip.push_str(
                "\nViewer Active — operate the viewer. Drag the name bar to move.",
            );
        }
        if body_resp.hovered() {
            body_resp.on_hover_text(&tip);
        }
        if name_resp.hovered() {
            name_resp.on_hover_text(tip);
        }
    }
    if let Some(DragState::Node { id: drag_id, grab }) = app.canvas.drag {
        if drag_id == id && !app.graph.node(id).flags.lock {
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                app.edit(&format!("move:{id:?}"));
                let world = app.canvas.view.to_world(origin, pointer) - grab;
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
                    app.graph.node_mut_quiet(other).pos = [p[0] + delta.x, p[1] + delta.y];
                }
                app.graph.node_mut_quiet(id).pos = snapped;
            }
        }
    }
    if let Some(DragState::Resize {
        id: drag_id,
        start_pointer,
        start_size,
    }) = app.canvas.drag
    {
        if drag_id == id && !locked {
            if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                app.edit(&format!("resize:{id:?}"));
                let world = app.canvas.view.to_world(origin, pointer);
                let delta = world - start_pointer;
                let w = (start_size.x + delta.x).max(crate::canvas::types::MIN_NODE_W);
                let h = (start_size.y + delta.y).max(crate::canvas::types::MIN_NODE_H);
                app.graph.node_mut_quiet(id).size = Some([w, h]);
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }
        }
    }
    let panel_interact = viewer_operate && !locked;
    let mut panel_edits = Vec::new();
    if !panel_local.is_empty() {
        panel_edits.extend(crate::app::draw_panel_comp(
            ui,
            app,
            id,
            body,
            panel_local,
            panel_interact,
        ));
    } else if otd_engine::panel::is_panel_type(op_label) {
        let w = otd_engine::panel::panel_display_widgets(&app.graph, id);
        if !w.is_empty() {
            panel_edits.extend(crate::app::draw_panel_comp(
                ui,
                app,
                id,
                body,
                &w,
                panel_interact,
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
    if viewer_on && family == Family::Dat && crate::params::dat_uses_text_param_editor(app, id) {
        crate::params::dat_node_viewer_editor(app, ui, id, body, zoom, viewer_operate);
    } else if viewer_operate && family == Family::Dat {
        handle_dat_viewer_input(app, ui, id, body);
    }
    if crate::canvas::callback_embed::node_has_callback(app, id) {
        let strip = fold_strip_rect(rect, main_h_world, zoom);
        let _ = draw_fold_control(app, ui, id, strip, zoom);
    }
}

fn resize_grip_rect(main_rect: Rect, zoom: f32) -> Rect {
    let g = (14.0 * zoom).clamp(12.0, 20.0);
    Rect::from_min_max(
        Pos2::new(main_rect.max.x - g, main_rect.max.y - g),
        main_rect.max,
    )
}

fn paint_resize_grip(ui: &egui::Ui, grip: Rect, zoom: f32, hot: bool) {
    let color = if hot {
        Color32::from_rgb(210, 214, 222)
    } else {
        Color32::from_rgb(120, 124, 134)
    };
    let stroke = Stroke::new((1.25 * zoom).clamp(1.0, 1.6), color);
    let inset = (3.0 * zoom).clamp(2.0, 5.0);
    let step = (3.2 * zoom).clamp(2.4, 4.0);
    for i in 0..3 {
        let o = inset + i as f32 * step;
        ui.painter().line_segment(
            [
                Pos2::new(grip.max.x - inset, grip.min.y + o),
                Pos2::new(grip.min.x + o, grip.max.y - inset),
            ],
            stroke,
        );
    }
}