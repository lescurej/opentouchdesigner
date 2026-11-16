//! Draw `containerCOMP` trees in Perform and floating View windows.

use egui::{Painter, Rect, TextureId, Ui};
use otd_core::{Graph, NodeId};
use otd_engine::panel::{Kind, Widget};

use crate::app::OtdApp;
use crate::canvas::comp_display;

pub fn widget_rect(area: Rect, w: &Widget) -> Rect {
    Rect::from_min_size(
        area.min + egui::vec2(w.rect[0] * area.width(), w.rect[1] * area.height()),
        egui::vec2(w.rect[2] * area.width(), w.rect[3] * area.height()),
    )
}

pub fn draw_container_comp(
    ui: &mut Ui,
    app: &mut OtdApp,
    comp: NodeId,
    area: Rect,
    widgets: &[Widget],
) {
    draw_container_look(&app.graph, comp, ui.painter(), area);
    draw_container_widgets(ui, app, widgets, area);
}

fn draw_container_look(graph: &Graph, comp: NodeId, painter: &Painter, area: Rect) {
    let node = graph.node(comp);
    let ctx = otd_core::EvalContext::default();
    let f = |key: &str, default: f64| {
        node.param(key)
            .map(|p| p.eval(&ctx).as_f64())
            .unwrap_or(default)
    };
    let opacity = f("opacity", 1.0) as f32;
    let alpha = (opacity * f("bgalpha", 1.0) as f32).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let color = egui::Color32::from_rgba_unmultiplied(
        (f("bgcolorr", 0.08) * 255.0) as u8,
        (f("bgcolorg", 0.09) * 255.0) as u8,
        (f("bgcolorb", 0.11) * 255.0) as u8,
        (alpha * 255.0) as u8,
    );
    painter.rect_filled(area, 0.0, color);
}

fn draw_widget_look(painter: &Painter, rect: Rect, w: &Widget) {
    let alpha = (w.opacity * w.bg[3]).clamp(0.0, 1.0);
    if alpha <= 0.001 {
        return;
    }
    let fill = egui::Color32::from_rgba_unmultiplied(
        (w.bg[0] * 255.0) as u8,
        (w.bg[1] * 255.0) as u8,
        (w.bg[2] * 255.0) as u8,
        (alpha * 255.0) as u8,
    );
    painter.rect_filled(rect, 4.0, fill);
}

pub fn draw_texture(painter: &Painter, rect: Rect, tid: TextureId, size: [u32; 2]) {
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let draw = if aspect > rect.width() / rect.height() {
        let h = rect.width() / aspect;
        Rect::from_center_size(rect.center(), egui::vec2(rect.width(), h))
    } else {
        let w = rect.height() * aspect;
        Rect::from_center_size(rect.center(), egui::vec2(w, rect.height()))
    };
    painter.image(
        tid,
        draw,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

fn draw_container_widgets(ui: &mut Ui, app: &mut OtdApp, widgets: &[Widget], area: Rect) {
    let mut sorted: Vec<&Widget> = widgets.iter().collect();
    sorted.sort_by(|a, b| {
        a.layer
            .partial_cmp(&b.layer)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for w in sorted {
        if !w.display {
            continue;
        }
        let rect = widget_rect(area, w);
        if !ui.max_rect().intersects(rect) {
            continue;
        }
        if matches!(w.kind, Kind::Container) {
            let nested = otd_engine::panel::panel_display_widgets(&app.graph, w.id);
            draw_container_comp(ui, app, w.id, rect, &nested);
            continue;
        }
        if let Some(top) = comp_display::comp_view_top(&app.graph, w.id) {
            if let Some((tid, size)) = app.thumbnail(top) {
                draw_texture(ui.painter(), rect, tid, size);
                continue;
            }
        }
        draw_widget_look(ui.painter(), rect, w);
    }
}
