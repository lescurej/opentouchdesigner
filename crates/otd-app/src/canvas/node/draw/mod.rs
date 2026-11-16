mod body;
mod interaction;
mod ports;

use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;
use crate::canvas::callback_embed::main_node_world_height;
use crate::canvas::comp_display::panel_widgets_local_to_comp;
use crate::canvas::flags::{
    NodeFlagsView, draw_footer_status_flags, draw_left_flag_strip, footer_status_rects,
};
use crate::canvas::layout::node_layout;
use crate::canvas::palette::family_color;
use crate::canvas::viewer_active::viewer_operating;

use body::paint_node_body;
use interaction::handle_node_interaction;
use ports::{paint_ports, wire_port_interaction};

pub(crate) fn draw_node(app: &mut OtdApp, ui: &mut egui::Ui, id: NodeId, rect: Rect, origin: Pos2) {
    let zoom = app.canvas.view.zoom;
    let main_h_world = main_node_world_height(app, id);
    let main_h_screen = main_h_world * zoom;
    let main_rect = Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + main_h_screen));
    let panel_local =
        if app.graph.node(id).flags.viewer && app.graph.node(id).family == Family::Comp {
            panel_widgets_local_to_comp(&app.graph, id)
        } else {
            Vec::new()
        };
    let panel_local_empty = panel_local.is_empty();
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
    let primary = app.canvas.selected == Some(id);
    let perform_out = app.perform_ui() == Some(id) && family == Family::Comp;
    let animated = app.snapshot_time_dependent(id);
    let cook_us = app.snapshot_last_cook_us(id);

    let fc = family_color(family);
    let viewer_operate = viewer_operating(ui, viewer_on, viewer_active);
    let layout = node_layout(main_rect, zoom);
    let body = if family == Family::Chop {
        layout.body
    } else {
        layout.body.shrink2(Vec2::new(1.0, 1.0))
    };
    let footer = layout.footer;

    let fill = crate::canvas::palette::node_fill(selected);
    let corner = (crate::canvas::palette::node_corner() * zoom).clamp(0.0, 16.0);
    painter.rect_filled(main_rect, corner, fill);
    if selected {
        let halo_pad = (3.0 * zoom).clamp(2.0, 6.0);
        let halo_w = (2.5 * zoom).clamp(2.0, 4.0);
        let halo_c = crate::canvas::palette::selection_color(primary);
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
            Rect::from_min_max(Pos2::new(body.max.x - rail_w, body.min.y), body.max),
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
        .canvas.canvas_rename
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

    paint_node_body(
        app,
        ui,
        id,
        family,
        &op_label,
        body,
        footer,
        zoom,
        viewer_on,
        bypassed,
        animated,
        cook_us,
        perform_out,
        name_field,
        panel_local_empty,
    );

    crate::canvas::node_message::paint_node_message(app, ui.painter(), id, main_rect, zoom);
    paint_ports(
        app,
        ui.painter(),
        id,
        main_rect,
        zoom,
        input_count,
        &input_slots,
        &input_families,
        family,
    );

    wire_port_interaction(
        app,
        ui,
        id,
        main_rect,
        zoom,
        locked,
        input_count,
        &input_slots,
    );

    handle_node_interaction(
        app,
        ui,
        id,
        rect,
        origin,
        main_rect,
        body,
        footer,
        name_field,
        layout.flags,
        zoom,
        locked,
        family,
        &op_label,
        viewer_on,
        viewer_operate,
        main_h_world,
        &panel_local,
    );
}