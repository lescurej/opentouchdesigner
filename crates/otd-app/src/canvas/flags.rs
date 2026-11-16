use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use otd_core::{Family, NodeId};

use crate::app::OtdApp;

use super::input::{SelectionFlag, toggle_flag_at};

pub(crate) struct NodeFlagsView {
    pub viewer: bool,
    pub clone_immune: bool,
    pub bypass: bool,
    pub lock: bool,
    pub bg_display: bool,
    pub viewer_active: bool,
}

const LEFT_FLAG_LABELS: [&str; 4] = ["V", "C", "B", "L"];

const LEFT_FLAG_COLORS: [(Color32, Color32); 4] = [
    (
        Color32::from_rgb(70, 140, 220),
        Color32::from_rgb(180, 220, 255),
    ),
    (
        Color32::from_rgb(200, 160, 70),
        Color32::from_rgb(255, 220, 150),
    ),
    (
        Color32::from_rgb(196, 168, 88),
        Color32::from_rgb(230, 210, 150),
    ),
    (
        Color32::from_rgb(200, 160, 70),
        Color32::from_rgb(255, 220, 150),
    ),
];

const LEFT_FLAG_COUNT: usize = 4;

pub(crate) fn flag_cell_rect(strip: Rect, index: usize, zoom: f32) -> Rect {
    let pad = 2.0 * zoom;
    let cell_h = ((strip.height() - pad * 2.0) / LEFT_FLAG_COUNT as f32).min(15.0 * zoom);
    let y = strip.min.y + pad + cell_h * index as f32;
    Rect::from_min_max(
        Pos2::new(strip.min.x + 1.0 * zoom, y),
        Pos2::new(strip.max.x - 1.0 * zoom, y + cell_h - 1.0 * zoom),
    )
}

pub(crate) fn display_flag_allowed(family: Family) -> bool {
    !matches!(family, Family::Comp | Family::Mat)
}

pub(crate) fn node_drag_rect(node: Rect, flag_strip: Rect, footer: Rect, zoom: f32) -> Rect {
    let gutter = (16.0 * zoom).clamp(12.0, 22.0);
    Rect::from_min_max(
        Pos2::new(flag_strip.max.x + gutter, node.min.y),
        Pos2::new(node.max.x - gutter, footer.min.y),
    )
}

pub(crate) fn footer_status_rects(footer: Rect, zoom: f32) -> (Rect, Rect, Rect) {
    let pad = 2.0 * zoom;
    let inner = Rect::from_min_max(
        Pos2::new(footer.min.x + pad, footer.min.y + pad),
        Pos2::new(footer.max.x - pad, footer.max.y - pad),
    );
    let cell = (14.0 * zoom).clamp(10.0, 18.0);
    let gap = (3.0 * zoom).clamp(2.0, 5.0);
    let viewer_active = Rect::from_min_size(
        Pos2::new(inner.max.x - cell, inner.center().y - cell * 0.5),
        Vec2::splat(cell),
    );
    let display = Rect::from_min_size(
        Pos2::new(
            viewer_active.min.x - gap - cell,
            inner.center().y - cell * 0.5,
        ),
        Vec2::splat(cell),
    );
    let name = Rect::from_min_max(inner.min, Pos2::new(display.min.x - gap, inner.max.y));
    (name, display, viewer_active)
}

fn paint_letter_tile(
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    on: bool,
    on_fill: Color32,
    on_text: Color32,
    zoom: f32,
) {
    let off_bg = Color32::from_rgb(26, 28, 32);
    let off_border = Color32::from_rgb(52, 54, 62);
    painter.rect_filled(
        rect,
        1.0,
        if on {
            on_fill.gamma_multiply(0.35)
        } else {
            off_bg
        },
    );
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
    painter.rect_filled(strip, 0.0, Color32::from_rgb(30, 32, 38));
    painter.rect_filled(
        Rect::from_min_size(strip.min, Vec2::new(2.0 * zoom, strip.height())),
        0.0,
        accent.gamma_multiply(0.85),
    );
    if zoom < 0.32 {
        return;
    }
    let states = [flags.viewer, flags.clone_immune, flags.bypass, flags.lock];
    for (i, label) in LEFT_FLAG_LABELS.iter().enumerate() {
        let (fill, text) = LEFT_FLAG_COLORS[i];
        paint_letter_tile(
            painter,
            flag_cell_rect(strip, i, zoom),
            label,
            states[i],
            fill,
            text,
            zoom,
        );
    }
}

pub(crate) fn draw_footer_status_flags(
    painter: &egui::Painter,
    footer: Rect,
    flags: &NodeFlagsView,
    zoom: f32,
    family: Family,
) {
    if zoom < 0.32 {
        return;
    }
    let (_name, display_rect, active_rect) = footer_status_rects(footer, zoom);
    if display_flag_allowed(family) {
        let display_on = flags.bg_display;
        paint_letter_tile(
            painter,
            display_rect,
            "D",
            display_on,
            Color32::from_rgb(80, 150, 255),
            Color32::from_rgb(200, 225, 255),
            zoom,
        );
    }
    paint_letter_tile(
        painter,
        active_rect,
        "A",
        flags.viewer_active,
        Color32::from_rgb(210, 190, 110),
        Color32::from_rgb(255, 240, 180),
        zoom,
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
        Stroke::new(
            (1.25 * zoom).clamp(1.0, 1.5),
            Color32::from_rgb(190, 162, 82),
        ),
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
    for i in 0..LEFT_FLAG_COUNT {
        if locked && i != 3 {
            continue;
        }
        let cell = flag_cell_rect(strip, i, zoom).expand(2.0);
        if click_left_flag(ui, id, i, cell, app) {
            used = true;
        }
    }
    used
}

pub(crate) fn handle_footer_flag_clicks(
    app: &mut OtdApp,
    ui: &mut Ui,
    id: NodeId,
    footer: Rect,
    locked: bool,
    zoom: f32,
    family: Family,
) -> bool {
    if zoom < 0.32 {
        return false;
    }
    let (_name, display_rect, active_rect) = footer_status_rects(footer, zoom);
    let mut used = false;
    if display_flag_allowed(family) {
        let resp = ui.interact(
            display_rect.expand(2.0),
            egui::Id::new(("flag_display", id)),
            Sense::click(),
        );
        if resp.clicked() && !locked {
            toggle_flag_at(app, SelectionFlag::Display, id);
            used = true;
        }
        resp.on_hover_text(if locked {
            "Display (D) — unlock the node to toggle network background mosaic"
        } else {
            "Display (D) — show this operator in the network background mosaic"
        });
    }
    if !locked {
        let resp = ui.interact(
            active_rect.expand(2.0),
            egui::Id::new(("flag_viewer_active", id)),
            Sense::click(),
        );
        if resp.clicked() {
            toggle_flag_at(app, SelectionFlag::ViewerActive, id);
            used = true;
        }
        resp.on_hover_text(
            "Viewer Active (A) — operate the node viewer; drag the name bar to move the node",
        );
    } else {
        let resp = ui.interact(
            active_rect.expand(2.0),
            egui::Id::new(("flag_viewer_active", id)),
            Sense::click(),
        );
        resp.on_hover_text("Viewer Active (A) — unlock the node to toggle");
    }
    used
}

fn click_left_flag(ui: &mut Ui, id: NodeId, index: usize, rect: Rect, app: &mut OtdApp) -> bool {
    let tooltips = [
        "Viewer (V) — show the operator viewer on the node body",
        "Clone Immune (C) — keep local changes when the clone master re-syncs",
        "Bypass (B) — pass input 0 straight through, do not run this op",
        "Lock (L) — block edits and wiring",
    ];
    let resp = ui.interact(rect, egui::Id::new(("flag", id, index)), Sense::click());
    if resp.clicked() {
        let flag = match index {
            0 => SelectionFlag::Viewer,
            1 => SelectionFlag::CloneImmune,
            2 => SelectionFlag::Bypass,
            3 => SelectionFlag::Lock,
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
