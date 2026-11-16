use super::*;

pub(crate) fn draw_texture_stretch(
    ui: &egui::Ui,
    tid: TextureId,
    size: [u32; 2],
    stretch: crate::window_layout::TextureStretch,
) -> egui::Rect {
    let area = ui.max_rect();
    let avail = area.size();
    let tex = egui::Vec2::new(size[0].max(1) as f32, size[1].max(1) as f32);
    let rect = match stretch {
        crate::window_layout::TextureStretch::Stretch => area,
        crate::window_layout::TextureStretch::Native => {
            let s = (avail.x / tex.x).min(avail.y / tex.y).min(1.0);
            let w = tex.x * s;
            let h = tex.y * s;
            egui::Rect::from_center_size(area.center(), egui::vec2(w, h))
        }
        crate::window_layout::TextureStretch::Fill => {
            let dst = avail.x / avail.y.max(1.0);
            let src = tex.x / tex.y;
            let (w, h) = if src > dst {
                (avail.x, avail.x / src)
            } else {
                (avail.y * src, avail.y)
            };
            let mut r = egui::Rect::from_center_size(area.center(), egui::vec2(w, h));
            if w > avail.x + 0.5 || h > avail.y + 0.5 {
                let scale = (avail.x / w).max(avail.y / h);
                r = egui::Rect::from_center_size(
                    area.center(),
                    egui::vec2(w * scale, h * scale),
                );
            }
            r
        }
        crate::window_layout::TextureStretch::Fit => {
            let aspect = tex.x / tex.y;
            let (mut w, mut h) = (avail.x, avail.x / aspect);
            if h > avail.y {
                h = avail.y;
                w = h * aspect;
            }
            egui::Rect::from_center_size(area.center(), egui::vec2(w, h))
        }
    };
    ui.painter().image(
        tid,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    rect
}

/// Paint a texture centred and letterboxed.
///
/// Letterbox rather than stretch, in both the output window and perform mode:
/// a show output at the wrong aspect ratio is worse than black bars, and worse
/// still because it looks deliberate.
/// Returns the rect it painted into, which is the box a panel lays out
/// against — the picture, not the window, so a widget stays where it was put
/// relative to the image whatever shape the window is.
pub(crate) fn letterbox(ui: &egui::Ui, tid: TextureId, size: [u32; 2]) -> egui::Rect {
    let avail = ui.available_size();
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let (mut w, mut h) = (avail.x, avail.x / aspect);
    if h > avail.y {
        h = avail.y;
        w = h * aspect;
    }
    let rect = egui::Rect::from_center_size(ui.max_rect().center(), egui::vec2(w, h));
    ui.painter().image(
        tid,
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    rect
}

/// Draw the panel widgets over `area`, returning the values a person changed.
///
/// The edits come back rather than being applied here: writing needs the graph
/// mutably, and the caller is already holding it to have drawn at all. Keeping
/// that split also means this function is only about pixels.
fn panel_widget_rect(area: egui::Rect, w: &otd_engine::panel::Widget) -> egui::Rect {
    egui::Rect::from_min_size(
        area.min + egui::vec2(w.rect[0] * area.width(), w.rect[1] * area.height()),
        egui::vec2(w.rect[2] * area.width(), w.rect[3] * area.height()),
    )
}

pub(crate) fn draw_panel_comp(
    ui: &mut egui::Ui,
    app: &mut OtdApp,
    comp: NodeId,
    area: egui::Rect,
    widgets: &[otd_engine::panel::Widget],
    interact: bool,
) -> Vec<(NodeId, f64)> {
    draw_panel_comp_look(&app.graph, comp, ui.painter(), area);
    draw_panel(ui, widgets, area, Some(app), interact)
}

pub(crate) fn draw_panel_comp_look(
    graph: &Graph,
    comp: NodeId,
    painter: &egui::Painter,
    area: egui::Rect,
) {
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

fn draw_panel_widget_look(painter: &egui::Painter, rect: egui::Rect, w: &otd_engine::panel::Widget) {
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

pub(crate) fn draw_panel_texture(
    painter: &egui::Painter,
    rect: egui::Rect,
    tid: TextureId,
    size: [u32; 2],
) {
    let aspect = size[0] as f32 / size[1].max(1) as f32;
    let draw = if aspect > rect.width() / rect.height() {
        let h = rect.width() / aspect;
        egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), h))
    } else {
        let w = rect.height() * aspect;
        egui::Rect::from_center_size(rect.center(), egui::vec2(w, rect.height()))
    };
    painter.image(
        tid,
        draw,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

pub(crate) fn draw_panel(
    ui: &mut egui::Ui,
    widgets: &[otd_engine::panel::Widget],
    area: egui::Rect,
    mut app: Option<&mut OtdApp>,
    interact: bool,
) -> Vec<(NodeId, f64)> {
    use otd_engine::panel::Kind;
    let mut sorted: Vec<&otd_engine::panel::Widget> = widgets.iter().collect();
    sorted.sort_by(|a, b| {
        a.layer
            .partial_cmp(&b.layer)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut hits: Vec<(otd_engine::panel::Widget, egui::Rect)> = Vec::new();
    let mut changed = Vec::new();
    for w in sorted {
        if !w.display {
            continue;
        }
        let rect = panel_widget_rect(area, w);
        if !ui.max_rect().intersects(rect) {
            continue;
        }
        if matches!(w.kind, Kind::Container) {
            if let Some(app) = app.as_mut() {
                let nested =
                    otd_engine::panel::panel_display_widgets(&app.graph, w.id);
                changed.extend(draw_panel_comp(
                    ui, app, w.id, rect, &nested, interact,
                ));
            } else {
                draw_panel_widget_look(ui.painter(), rect, w);
            }
            continue;
        }
        if let Some(app) = app.as_mut() {
            if let Some(top) = crate::canvas::comp_display::comp_view_top(&app.graph, w.id) {
                if let Some((tid, size)) = app.thumbnail(top) {
                    draw_panel_texture(ui.painter(), rect, tid, size);
                }
            }
        }
        if interact && w.enabled {
            hits.push(((*w).clone(), rect));
        }
    }
    if !interact {
        return changed;
    }
    for (w, rect) in hits.into_iter().rev() {
        let child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        match w.kind {
            Kind::Button => {
                let on = w.value >= 0.5;
                let r = child.interact(
                    rect,
                    child.id().with(("panel-btn", w.id)),
                    egui::Sense::click(),
                );
                if w.momentary {
                    let down = r.is_pointer_button_down_on();
                    if down != on {
                        changed.push((w.id, if down { 1.0 } else { 0.0 }));
                    }
                } else if r.clicked() {
                    changed.push((w.id, if on { 0.0 } else { 1.0 }));
                }
            }
            Kind::Slider => {
                let (lo, hi) = w.range;
                let r = child.interact(
                    rect,
                    child.id().with(("panel-slider", w.id)),
                    egui::Sense::click_and_drag(),
                );
                if r.clicked() || r.dragged() {
                    let t = if w.vertical {
                        1.0 - ((r.interact_pointer_pos().unwrap_or(rect.center()).y - rect.min.y)
                            / rect.height())
                    } else {
                        (r.interact_pointer_pos().unwrap_or(rect.center()).x - rect.min.x)
                            / rect.width()
                    };
                    let value = lo + (hi - lo) * t.clamp(0.0, 1.0) as f64;
                    changed.push((w.id, value));
                }
            }
            _ => {}
        }
    }
    changed
}
