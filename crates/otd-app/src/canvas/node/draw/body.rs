use egui::{Align2, Color32, FontId, Pos2, Rect, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;
use crate::canvas::bodies::{draw_dat_body, draw_sop_body};
use crate::canvas::chop::draw_chop_body;
use crate::canvas::comp_display::comp_view_top;
use crate::canvas::flags::{draw_bypass_body_fx, family_icon_label};
use crate::canvas::palette::family_color;
use crate::canvas::viewer_active::{top_viewer_screen_rect_for, viewer_operating};

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_node_body(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: NodeId,
    family: Family,
    op_label: &str,
    body: Rect,
    footer: Rect,
    zoom: f32,
    viewer_on: bool,
    bypassed: bool,
    animated: bool,
    cook_us: u64,
    perform_out: bool,
    name_field: Rect,
    panel_local_empty: bool,
) {
    let fc = family_color(family);
    let viewer_operate = viewer_operating(ui, viewer_on, app.graph.node(id).flags.viewer_active);
    let cached_preview = viewer_on
        && !animated
        && app.snapshot_has_cooked(id)
        && !app.snapshot_cooked_this_frame(id);
    if !viewer_on && zoom > 0.3 {
        let icon = family_icon_label(family, op_label);
        let painter = ui.painter();
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
                    ui.painter().with_clip_rect(body).image(
                        tid,
                        draw,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
            }
            Family::Chop => draw_chop_body(app, ui, id, body, zoom),
            Family::Dat => {
                if viewer_on && crate::params::dat_uses_text_param_editor(app, id) {
                    ui.painter()
                        .rect_filled(body, 0.0, Color32::from_rgb(22, 23, 26));
                } else {
                    draw_dat_body(
                        app,
                        ui,
                        id,
                        body,
                        zoom,
                        if viewer_operate {
                            crate::canvas::viewer_active::dat_viewer_scroll(app, id)
                        } else {
                            Vec2::ZERO
                        },
                    );
                }
            }
            Family::Sop => draw_sop_body(app, ui, id, body, zoom),
            Family::Comp
                if panel_local_empty && !otd_engine::panel::is_panel_type(op_label) =>
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
            _ => {}
        }
    }
    let painter = ui.painter();
    if cached_preview {
        painter.rect_filled(body, 2.0, Color32::from_black_alpha(90));
    }
    if bypassed {
        draw_bypass_body_fx(painter, body, zoom);
    }
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
}