use crate::app::OtdApp;

const MAX_NAME: usize = 20;

pub(crate) fn sanitise(hint: &str) -> Option<String> {
    let mut out = String::new();
    let mut last_underscore = false;
    for c in hint.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_underscore = false;
        } else if !last_underscore && !out.is_empty() {
            out.push('_');
            last_underscore = true;
        }
    }
    if out.len() > MAX_NAME {
        let head = &out[..MAX_NAME];
        out = match head.rfind('_') {
            Some(cut) if cut >= MAX_NAME / 2 => head[..cut].to_string(),
            _ => head.to_string(),
        };
    }
    let out = out.trim_end_matches('_').to_string();
    if out.is_empty() || !out.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(out)
}

pub(crate) fn default_pos(app: &OtdApp) -> egui::Vec2 {
    if app.canvas.canvas_rect.is_positive() {
        app.canvas.view
            .to_world(app.canvas.canvas_rect.min, app.canvas.canvas_rect.center())
            - egui::vec2(crate::canvas::NODE_W, crate::canvas::NODE_H) * 0.5
    } else {
        egui::Vec2::ZERO
    }
}

pub(crate) fn free_spot(app: &OtdApp, mut pos: egui::Vec2) -> egui::Vec2 {
    let occupied = |app: &OtdApp, pos: egui::Vec2| {
        app.graph.children(app.canvas.current).iter().any(|id| {
            let p = app.graph.node(*id).pos;
            (p[0] - pos.x).abs() < crate::canvas::NODE_W * 0.5
                && (p[1] - pos.y).abs() < crate::canvas::NODE_H * 0.5
        })
    };
    let mut guard = 0;
    while occupied(app, pos) && guard < 64 {
        pos.y += crate::canvas::NODE_H + 28.0;
        guard += 1;
    }
    pos
}