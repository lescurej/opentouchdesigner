use egui::{Color32, Painter, Rect, Stroke};

use otd_engine::PythonLint;

pub(crate) fn code_pixel_width(text: &str, char_w: f32) -> f32 {
    let max_chars = text
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);
    max_chars as f32 * char_w + 32.0
}

pub(crate) fn indent_guide_columns(text: &str) -> Vec<usize> {
    let tab = 4;
    let mut cols = std::collections::BTreeSet::new();
    for line in text.lines() {
        let mut col = 0usize;
        for c in line.chars() {
            match c {
                ' ' => col += 1,
                '\t' => col += tab,
                _ => break,
            }
        }
        let mut t = tab;
        while t < col {
            cols.insert(t);
            t += tab;
        }
    }
    cols.into_iter().collect()
}

pub(crate) fn paint_indent_guides(painter: &Painter, rect: Rect, char_w: f32, columns: &[usize]) {
    if columns.is_empty() {
        return;
    }
    let color = Color32::from_rgba_unmultiplied(90, 100, 120, 100);
    let stroke = Stroke::new(1.0, color);
    let clip = painter.with_clip_rect(rect);
    for &col in columns {
        let x = rect.min.x + col as f32 * char_w;
        if x >= rect.max.x {
            break;
        }
        clip.line_segment(
            [egui::pos2(x, rect.min.y), egui::pos2(x, rect.max.y)],
            stroke,
        );
    }
}

pub(crate) fn paint_issue_squiggles(
    painter: &Painter,
    text_rect: Rect,
    row_h: f32,
    char_w: f32,
    issues: &[PythonLint],
    text: &str,
) {
    for issue in issues {
        let line_idx = issue.line as usize;
        if line_idx == 0 {
            continue;
        }
        let Some(line_text) = text.lines().nth(line_idx - 1) else {
            continue;
        };
        let y = text_rect.min.y + (line_idx - 1) as f32 * row_h + row_h - 2.0;
        let x0 = text_rect.min.x + issue.column as f32 * char_w;
        let span = if issue.span_len > 0 {
            issue.span_len as f32 * char_w
        } else {
            line_text.chars().count().max(1) as f32 * char_w
        };
        let x1 = (x0 + span).min(text_rect.max.x);
        if x1 <= x0 {
            continue;
        }
        let color = match issue.severity {
            otd_engine::LintSeverity::Error => Color32::from_rgb(230, 110, 110),
            otd_engine::LintSeverity::Warning => Color32::from_rgb(230, 190, 110),
        };
        paint_wavy_line(painter, x0, x1, y, color);
    }
}

fn paint_wavy_line(painter: &Painter, x0: f32, x1: f32, y: f32, color: Color32) {
    let stroke = Stroke::new(1.0, color);
    let amp = 1.5;
    let wave = 4.0;
    let mut x = x0;
    let mut up = true;
    while x < x1 {
        let nx = (x + wave).min(x1);
        let y1 = if up { y - amp } else { y + amp };
        painter.line_segment([egui::pos2(x, y), egui::pos2(nx, y1)], stroke);
        x = nx;
        up = !up;
    }
}