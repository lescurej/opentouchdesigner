use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};
use otd_chop::ChopData;



pub(crate) fn chop_axis_font(zoom: f32) -> FontId {
    FontId::proportional((8.0 * zoom).clamp(7.0, 11.0))
}

pub(crate) fn chop_lane_label_font(zoom: f32) -> FontId {
    FontId::proportional((11.0 * zoom).clamp(9.0, 16.0))
}

pub(crate) fn format_channel_value(v: f32) -> String {
    format_axis_number(v)
}

pub(crate) fn paint_chop_text(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: impl AsRef<str>,
    font: FontId,
    fg: Color32,
) {
    let text = text.as_ref();
    let shadow = Color32::from_rgba_unmultiplied(0, 0, 0, 200);
    for (dx, dy) in [(1.0, 1.0), (1.0, 0.0), (0.0, 1.0)] {
        painter.text(pos + Vec2::new(dx, dy), align, text, font.clone(), shadow);
    }
    painter.text(pos, align, text, font, fg);
}

pub(crate) fn format_axis_number(v: f32) -> String {
    if !v.is_finite() {
        return "?".into();
    }
    let av = v.abs();
    if av >= 1000.0 || (av > 0.0 && av < 0.001) {
        format!("{v:.2e}")
    } else if av >= 100.0 {
        format!("{v:.0}")
    } else if (v - v.round()).abs() < 1e-3 {
        format!("{}", v.round() as i64)
    } else if av <= 2.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

pub(crate) fn format_x_sample_index(i: usize) -> String {
    format!("{i}i")
}

pub(crate) fn nice_step(raw: f32) -> f32 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let exp = raw.log10().floor();
    let base = 10f32.powf(exp);
    let frac = raw / base;
    let nice_frac = if frac <= 1.0 {
        1.0
    } else if frac <= 2.0 {
        2.0
    } else if frac <= 5.0 {
        5.0
    } else {
        10.0
    };
    nice_frac * base
}

pub(crate) fn nice_value_ticks(lo: f32, hi: f32, max_ticks: usize) -> Vec<f32> {
    if !lo.is_finite() || !hi.is_finite() {
        return vec![0.0];
    }
    let span = (hi - lo).max(1e-12);
    let step = nice_step(span / max_ticks.max(2) as f32);
    let start = (lo / step).floor() * step;
    let mut ticks = Vec::new();
    let mut v = start;
    let end = hi + step * 0.001;
    while v <= end {
        if v >= lo - step * 0.001 && v <= hi + step * 0.001 {
            ticks.push(v);
        }
        v += step;
        if ticks.len() > max_ticks + 2 {
            break;
        }
    }
    if ticks.is_empty() {
        ticks.push(lo);
        if (hi - lo).abs() > 1e-6 {
            ticks.push(hi);
        }
    }
    ticks.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    ticks.dedup_by(|a, b| (*a - *b).abs() < step * 0.01);
    ticks
}

pub(crate) fn sample_index_ticks(ns: usize, max_ticks: usize) -> Vec<usize> {
    if ns == 0 {
        return Vec::new();
    }
    let last = ns.saturating_sub(1);
    if last == 0 {
        return vec![0];
    }
    let step_f = nice_step(last as f32 / max_ticks.max(2) as f32);
    let step = step_f.round().max(1.0) as usize;
    let mut out = Vec::new();
    let mut i = 0;
    while i <= last {
        out.push(i);
        i = i.saturating_add(step);
        if step == 0 {
            break;
        }
    }
    if out.last() != Some(&last) {
        out.push(last);
    }
    out.sort_unstable();
    out.dedup();
    out
}

pub(crate) fn x_plot_x(plot: Rect, index: usize, last: usize) -> f32 {
    let t = index as f32 / last.max(1) as f32;
    plot.min.x + t * plot.width()
}

pub(crate) fn chop_plot_layout(body: Rect, zoom: f32, y_axis: bool) -> (Rect, Rect, Rect) {
    let left = if y_axis {
        (28.0 * zoom).clamp(22.0, 36.0)
    } else {
        (4.0 * zoom).clamp(2.0, 6.0)
    };
    let bottom = (12.0 * zoom).clamp(10.0, 16.0);
    let y_axis = Rect::from_min_max(body.min, Pos2::new(body.min.x + left, body.max.y - bottom));
    let x_axis = Rect::from_min_max(Pos2::new(body.min.x + left, body.max.y - bottom), body.max);
    let plot = Rect::from_min_max(
        Pos2::new(body.min.x + left, body.min.y),
        Pos2::new(body.max.x, body.max.y - bottom),
    );
    (y_axis, x_axis, plot)
}

pub(crate) fn draw_chop_plot_grid(
    painter: &egui::Painter,
    plot: Rect,
    y_ticks: &[f32],
    x_indices: &[usize],
    last: usize,
    lo: f32,
    hi: f32,
) {
    let grid = Color32::from_rgb(48, 50, 56);
    let span = (hi - lo).max(1e-12);
    for &v in y_ticks {
        let t = ((v - lo) / span).clamp(0.0, 1.0);
        let y = plot.max.y - t * plot.height();
        painter.line_segment(
            [Pos2::new(plot.min.x, y), Pos2::new(plot.max.x, y)],
            Stroke::new(1.0, grid),
        );
    }
    for &i in x_indices {
        let x = x_plot_x(plot, i, last);
        painter.line_segment(
            [Pos2::new(x, plot.min.y), Pos2::new(x, plot.max.y)],
            Stroke::new(1.0, grid),
        );
    }
}

pub(crate) fn draw_chop_y_axis(painter: &egui::Painter, axis: Rect, plot: Rect, lo: f32, hi: f32, zoom: f32) {
    let font = chop_axis_font(zoom);
    let color = Color32::from_rgb(130, 132, 140);
    let tick = Color32::from_rgb(58, 60, 66);
    let values = nice_value_ticks(lo, hi, 5);
    let span = (hi - lo).max(1e-12);
    for v in values {
        let t = ((v - lo) / span).clamp(0.0, 1.0);
        let y = plot.min.y + (1.0 - t) * plot.height();
        painter.line_segment(
            [Pos2::new(axis.max.x - 3.0, y), Pos2::new(axis.max.x, y)],
            Stroke::new(1.0, tick),
        );
        painter.text(
            Pos2::new(axis.max.x - 4.0, y),
            Align2::RIGHT_CENTER,
            format_axis_number(v),
            font.clone(),
            color,
        );
    }
}

pub(crate) fn draw_chop_x_axis(painter: &egui::Painter, axis: Rect, plot: Rect, ns: usize, zoom: f32) {
    if ns == 0 {
        return;
    }
    let font = chop_axis_font(zoom);
    let color = Color32::from_rgb(130, 132, 140);
    let tick = Color32::from_rgb(58, 60, 66);
    let last = ns.saturating_sub(1);
    let max_ticks = ((plot.width() / (36.0 * zoom).clamp(28.0, 44.0)).floor() as usize).clamp(3, 8);
    let indices = sample_index_ticks(ns, max_ticks);
    for i in indices {
        if i >= ns {
            continue;
        }
        let x = x_plot_x(plot, i, last);
        painter.line_segment(
            [Pos2::new(x, axis.min.y), Pos2::new(x, axis.min.y + 3.0)],
            Stroke::new(1.0, tick),
        );
        let label = format_x_sample_index(i);
        let align = if i == 0 {
            Align2::LEFT_TOP
        } else if i >= last {
            Align2::RIGHT_TOP
        } else {
            Align2::CENTER_TOP
        };
        painter.text(
            Pos2::new(x, axis.min.y + 2.0),
            align,
            label,
            font.clone(),
            color,
        );
    }
}

pub(crate) fn displayed_sample_count(data: &ChopData) -> usize {
    data.num_samples().min(super::viewer_limits().max_samples_scan)
}

pub(crate) fn pad_value_bounds(min_v: f32, max_v: f32) -> (f32, f32) {
    if !min_v.is_finite() || !max_v.is_finite() {
        return (-1.0, 1.0);
    }
    if (max_v - min_v).abs() < 1e-12 {
        let c = min_v;
        let pad = c.abs().max(1.0) * 0.05 + 0.01;
        return (c - pad, c + pad);
    }
    let pad = (max_v - min_v) * 0.08;
    (min_v - pad, max_v + pad)
}

pub(crate) fn lane_span(
    samples: &[f32],
    columns: usize,
    max_scan: usize,
) -> Option<(f32, f32, Vec<(f32, f32)>)> {
    let n = samples.len();
    if n == 0 || columns == 0 {
        return None;
    }
    let cols = columns.min(n).max(1);
    let step = (n / max_scan.max(cols)).max(1);
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    let mut out = Vec::with_capacity(cols);
    for c in 0..cols {
        let start = c * n / cols;
        let end = if c + 1 == cols {
            n
        } else {
            ((c + 1) * n / cols).max(start + 1)
        };
        let mut cmin = f32::INFINITY;
        let mut cmax = f32::NEG_INFINITY;
        let mut i = start;
        while i < end {
            let v = samples[i];
            if v.is_finite() {
                cmin = cmin.min(v);
                cmax = cmax.max(v);
            }
            i = i.saturating_add(step);
        }
        let v = samples[end - 1];
        if v.is_finite() {
            cmin = cmin.min(v);
            cmax = cmax.max(v);
        }
        if !cmin.is_finite() {
            cmin = 0.0;
            cmax = 0.0;
        }
        lo = lo.min(cmin);
        hi = hi.max(cmax);
        out.push((cmin, cmax));
    }
    if !lo.is_finite() {
        return None;
    }
    Some((lo, hi, out))
}

pub(crate) fn chop_value_bounds_channels(
    data: &ChopData,
    channel_indices: &[usize],
    sample_count: usize,
) -> (f32, f32) {
    let mut min_v = f32::INFINITY;
    let mut max_v = f32::NEG_INFINITY;
    let n = sample_count.min(displayed_sample_count(data));
    for &ci in channel_indices {
        let Some(ch) = data.channels.get(ci) else {
            continue;
        };
        for &v in ch.samples.iter().take(n) {
            if v.is_finite() {
                min_v = min_v.min(v);
                max_v = max_v.max(v);
            }
        }
    }
    pad_value_bounds(min_v, max_v)
}

#[cfg(test)]
mod tests {
    use super::lane_span;

    #[test]
    fn lane_span_keeps_each_end_of_a_long_buffer() {
        let mut samples = vec![0.0; 10_000];
        samples[0] = -5.0;
        samples[9_999] = 8.0;
        let (lo, hi, cols) = lane_span(&samples, 8, 64).unwrap();
        assert!((lo - -5.0).abs() < 1e-5);
        assert!((hi - 8.0).abs() < 1e-5);
        assert!(cols.first().unwrap().0 <= -5.0 + 1e-5);
        assert!(cols.last().unwrap().1 >= 8.0 - 1e-5);
    }
}

pub(crate) fn chop_bipolar_peak(data: &ChopData, channels: usize, samples: usize) -> f32 {
    let indices: Vec<usize> = (0..channels.min(data.channels.len())).collect();
    let (lo, hi) = chop_value_bounds_channels(data, &indices, samples);
    lo.abs().max(hi.abs()).max(1e-6)
}
