use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;

use super::input::{SelectionFlag, toggle_flag_at};

pub(crate) struct NodeFlagsView {
    pub viewer: bool,
    pub bg_display: bool,
    pub bypass: bool,
    pub lock: bool,
}

const LEFT_FLAGS: [&str; 4] = ["V", "D", "L", "B"];

pub(crate) fn flag_cell_rect(strip: Rect, index: usize, zoom: f32) -> Rect {
    let n = LEFT_FLAGS.len();
    let pad = 2.0 * zoom;
    let cell_h = ((strip.height() - pad * 2.0) / n as f32).min(15.0 * zoom);
    let y = strip.min.y + pad + cell_h * index as f32;
    Rect::from_min_max(
        Pos2::new(strip.min.x + 1.0 * zoom, y),
        Pos2::new(strip.max.x - 1.0 * zoom, y + cell_h - 1.0 * zoom),
    )
}

pub(crate) fn node_drag_rect(node: Rect, flag_strip: Rect, zoom: f32) -> Rect {
    let gutter = (16.0 * zoom).clamp(12.0, 22.0);
    Rect::from_min_max(
        Pos2::new(flag_strip.max.x + gutter, node.min.y),
        Pos2::new(node.max.x - gutter, node.max.y),
    )
}

fn paint_flag_button(
    painter: &egui::Painter,
    rect: Rect,
    on: bool,
    label: &str,
    on_fill: Color32,
    on_text: Color32,
    zoom: f32,
) {
    let off_bg = Color32::from_rgb(26, 28, 32);
    let off_border = Color32::from_rgb(52, 54, 62);
    painter.rect_filled(rect, 1.0, if on { on_fill.gamma_multiply(0.35) } else { off_bg });
    painter.rect_stroke(
        rect,
        1.0,
        Stroke::new(1.0, if on { on_fill } else { off_border }),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional((9.5 * zoom).clamp(7.0, 11.0)),
        if on {
            on_text
        } else {
            Color32::from_rgb(100, 102, 112)
        },
    );
}

pub(crate) fn draw_left_flag_strip(
    painter: &egui::Painter,
    strip: Rect,
    accent: Color32,
    flags: &NodeFlagsView,
    zoom: f32,
) {
    let colors = [
        (Color32::from_rgb(70, 140, 220), Color32::from_rgb(180, 220, 255)),
        (Color32::from_rgb(90, 170, 120), Color32::from_rgb(180, 240, 200)),
        (Color32::from_rgb(200, 160, 70), Color32::from_rgb(255, 220, 150)),
        (Color32::from_rgb(220, 120, 70), Color32::from_rgb(255, 200, 150)),
    ];
    let states = [
        flags.viewer,
        flags.bg_display,
        flags.lock,
        flags.bypass,
    ];
    painter.rect_filled(strip, 0.0, Color32::from_rgb(30, 32, 38));
    painter.rect_filled(
        Rect::from_min_size(strip.min, Vec2::new(2.0 * zoom, strip.height())),
        0.0,
        accent.gamma_multiply(0.85),
    );
    if zoom < 0.32 {
        return;
    }
    for (i, label) in LEFT_FLAGS.iter().enumerate() {
        let cell = flag_cell_rect(strip, i, zoom);
        if i == 3 {
            paint_bypass_flag(painter, cell, states[i], zoom);
        } else {
            let (fill, text) = colors[i];
            paint_flag_button(painter, cell, states[i], label, fill, text, zoom);
        }
    }
}

fn paint_bypass_flag(painter: &egui::Painter, rect: Rect, on: bool, zoom: f32) {
    let off_bg = Color32::from_rgb(26, 28, 32);
    let off_border = Color32::from_rgb(52, 54, 62);
    let on_border = Color32::from_rgb(196, 168, 88);
    let on_text = Color32::from_rgb(230, 210, 150);
    painter.rect_filled(
        rect,
        1.0,
        if on {
            Color32::from_rgb(38, 36, 30)
        } else {
            off_bg
        },
    );
    painter.rect_stroke(
        rect,
        1.0,
        Stroke::new(1.0, if on { on_border } else { off_border }),
        StrokeKind::Inside,
    );
    if on {
        let inset = 2.0 * zoom;
        painter.line_segment(
            [
                rect.min + Vec2::new(inset, inset),
                rect.max - Vec2::new(inset, inset),
            ],
            Stroke::new((1.25 * zoom).clamp(1.0, 1.5), on_border.gamma_multiply(0.85)),
        );
    }
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "B",
        FontId::proportional((9.5 * zoom).clamp(7.0, 11.0)),
        if on {
            on_text
        } else {
            Color32::from_rgb(100, 102, 112)
        },
    );
}

pub(crate) fn draw_bypass_body_fx(painter: &egui::Painter, body: Rect, zoom: f32) {
    if body.width() < 2.0 || body.height() < 2.0 {
        return;
    }
    let p = painter.with_clip_rect(body);
    p.rect_filled(body, 0.0, Color32::from_rgba_unmultiplied(18, 17, 14, 100));
    let inset = (3.0 * zoom).clamp(2.0, 6.0);
    let a = body.min + Vec2::splat(inset);
    let b = body.max - Vec2::splat(inset);
    let stroke = Stroke::new(
        (1.25 * zoom).clamp(1.0, 1.5),
        Color32::from_rgba_unmultiplied(210, 178, 95, 140),
    );
    p.line_segment([a, b], stroke);
    p.rect_stroke(
        body,
        0.0,
        Stroke::new((1.25 * zoom).clamp(1.0, 1.5), Color32::from_rgb(190, 162, 82)),
        StrokeKind::Inside,
    );
}

pub(crate) fn family_icon_label(family: Family, op_type: &str) -> String {
    let family_tag = match family {
        Family::Top => "TOP",
        Family::Chop => "CHOP",
        Family::Sop => "SOP",
        Family::Dat => "DAT",
        Family::Mat => "MAT",
        Family::Comp => "COMP",
    };
    let short = op_type
        .trim_end_matches(family.suffix())
        .chars()
        .take(8)
        .collect::<String>();
    if short.is_empty() {
        family_tag.to_string()
    } else {
        format!("{family_tag}\n{short}")
    }
}

pub(crate) fn handle_flag_clicks(
    app: &mut OtdApp,
    ui: &mut Ui,
    id: NodeId,
    strip: Rect,
    locked: bool,
    zoom: f32,
) -> bool {
    if zoom < 0.32 {
        return false;
    }
    let mut used = false;
    for i in 0..LEFT_FLAGS.len() {
        if locked && i != 2 {
            continue;
        }
        let cell = flag_cell_rect(strip, i, zoom).expand(2.0);
        if click_left_flag(ui, id, i, cell, app) {
            used = true;
        }
    }
    used
}

fn click_left_flag(ui: &mut Ui, id: NodeId, index: usize, rect: Rect, app: &mut OtdApp) -> bool {
    let tooltips = [
        "Viewer (V) — preview on the node body",
        "Display (D) — COMP only: tile on the network background",
        "Lock (L) — block edits and wiring",
        "Bypass (B) — pass input 0 straight through, do not run this op",
    ];
    let resp = ui.interact(
        rect,
        egui::Id::new(("flag", id, index)),
        Sense::click(),
    );
    if resp.clicked() {
        let flag = match index {
            0 => SelectionFlag::Viewer,
            1 => SelectionFlag::Display,
            2 => SelectionFlag::Lock,
            3 => SelectionFlag::Bypass,
            _ => return false,
        };
        toggle_flag_at(app, flag, id);
        return true;
    }
    if resp.hovered() {
        resp.on_hover_text(tooltips[index]);
    }
    false
}
