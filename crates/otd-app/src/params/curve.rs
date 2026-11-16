use egui::{Color32, RichText};
use otd_core::Value;

use crate::app::OtdApp;

pub(crate) fn curve_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    use otd_chop::anim::{Curves, Interp};

    let text = app.graph.node(id).params["keys"].value.as_str();
    let (mut curves, problems) = Curves::parse(&text);

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Keys");
        ui.label(
            RichText::new("click to add · drag to move · shift-click to delete")
                .weak()
                .small(),
        );
    });

    // The horizontal axis is the timeline's loop range, so the curve editor
    // and the scrub bar below it line up and the playhead means the same thing
    // in both.
    let (t0, t1) = app.transport.loop_range;
    let span = (t1 - t0).max(1e-6);
    let (lo, hi) = value_range(&curves);

    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 120.0),
        egui::Sense::click_and_drag(),
    );
    let to_screen = |t: f64, v: f32| {
        egui::pos2(
            rect.left() + ((t - t0) / span) as f32 * rect.width(),
            rect.bottom() - ((v - lo) / (hi - lo)) * rect.height(),
        )
    };
    let from_screen = |p: egui::Pos2| {
        (
            t0 + ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * span,
            lo + (1.0 - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0)) * (hi - lo),
        )
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, Color32::from_gray(26));
    // The zero line, when it is in view — the reference nearly every curve is
    // read against.
    if lo < 0.0 && hi > 0.0 {
        let y = to_screen(t0, 0.0).y;
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, Color32::from_gray(52)),
        );
    }

    let mut changed = false;
    let names: Vec<String> = curves.0.keys().cloned().collect();
    for (n, name) in names.iter().enumerate() {
        let tint = channel_color(n);
        let curve = &curves.0[name];

        // Sample per pixel rather than drawing straight lines between keys:
        // an eased or splined segment is a curve, and drawing it as a chord
        // would show a shape the render does not have.
        let steps = rect.width().max(2.0) as usize;
        let points: Vec<egui::Pos2> = (0..=steps)
            .map(|i| {
                let t = t0 + span * (i as f64 / steps as f64);
                to_screen(t, curve.sample(t))
            })
            .collect();
        painter.add(egui::Shape::line(points, egui::Stroke::new(1.5, tint)));

        for key in &curve.keys {
            painter.circle_filled(to_screen(key.time, key.value), 3.5, tint);
        }
    }

    // Editing acts on the channel the panel is focused on, which is the first
    // one unless the node has none yet.
    let target = names.first().cloned().unwrap_or_else(|| "chan1".into());

    if let Some(pos) = resp.interact_pointer_pos() {
        let (t, v) = from_screen(pos);
        let shift = ui.ctx().input(|i| i.modifiers.shift);
        // The nearest key within grabbing distance, in screen space so the
        // tolerance is the same however the axes are scaled.
        let hit = names.iter().find_map(|name| {
            curves.0[name]
                .keys
                .iter()
                .find(|k| to_screen(k.time, k.value).distance(pos) < 8.0)
                .map(|k| (name.clone(), k.time, k.interp))
        });

        if resp.drag_started() || resp.clicked() {
            match (&hit, shift) {
                (Some((name, time, _)), true) => {
                    app.edit("delete key");
                    app.document.history.end_gesture();
                    curves.0.get_mut(name).unwrap().remove_at(*time);
                    changed = true;
                }
                (None, false) => {
                    app.edit("add key");
                    app.document.history.end_gesture();
                    curves
                        .0
                        .entry(target.clone())
                        .or_default()
                        .set(t, v, Interp::Smooth);
                    changed = true;
                }
                _ => {}
            }
        } else if resp.dragged() {
            if let Some((name, time, interp)) = hit {
                // One undo entry for the whole drag: the tag names the key.
                app.edit(&format!("key:{name}:{time}"));
                let curve = curves.0.get_mut(&name).unwrap();
                curve.remove_at(time);
                curve.set(t, v, interp);
                changed = true;
            }
        }
    }
    if resp.drag_stopped() {
        app.document.history.end_gesture();
    }

    // The playhead, so a key can be placed against what is on screen.
    let x = to_screen(app.time.time, 0.0).x;
    if rect.x_range().contains(x) {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, Color32::from_rgb(230, 170, 90)),
        );
    }

    if changed {
        let _ = app
            .graph
            .set_param(id, "keys", Value::Str(curves.to_text()));
    }

    ui.horizontal(|ui| {
        for (n, name) in names.iter().enumerate() {
            ui.colored_label(channel_color(n), RichText::new(name).small());
        }
        ui.label(
            RichText::new(format!("{:.2} … {:.2}", lo, hi))
                .weak()
                .small(),
        );
    });

    // A hand-typed key with a typo is skipped rather than fatal, so the panel
    // has to say which line was dropped — otherwise it silently does nothing.
    for problem in &problems {
        ui.colored_label(
            Color32::from_rgb(235, 170, 120),
            RichText::new(problem).small(),
        );
    }

    let mut text = text;
    if ui
        .add(
            egui::TextEdit::multiline(&mut text)
                .code_editor()
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        )
        .changed()
    {
        app.edit(&format!("keys:{id:?}"));
        let _ = app.graph.set_param(id, "keys", Value::Str(text));
    }
    ui.add_space(4.0);
}
pub(crate) fn value_range(curves: &otd_chop::anim::Curves) -> (f32, f32) {
    let mut lo = f32::MAX;
    let mut hi = f32::MIN;
    for curve in curves.0.values() {
        for key in &curve.keys {
            lo = lo.min(key.value);
            hi = hi.max(key.value);
        }
    }
    if lo > hi {
        // No keys yet. 0..1 is the range most parameters live in, so a first
        // click lands somewhere useful.
        return (0.0, 1.0);
    }
    // A flat curve has no extent of its own; give it one rather than dividing
    // by zero and drawing a line off the top of the box.
    let pad = ((hi - lo) * 0.15).max(0.05);
    (lo - pad, hi + pad)
}
pub(crate) fn channel_color(n: usize) -> Color32 {
    const PALETTE: [Color32; 6] = [
        Color32::from_rgb(150, 200, 255),
        Color32::from_rgb(150, 220, 150),
        Color32::from_rgb(230, 170, 90),
        Color32::from_rgb(220, 140, 200),
        Color32::from_rgb(200, 200, 130),
        Color32::from_rgb(160, 160, 190),
    ];
    PALETTE[n % PALETTE.len()]
}
