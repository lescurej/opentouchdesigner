use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("off", &["off", "clamp", "loop", "zigzag", "quantise"])
            .with_label("Type"),
        "min" => Param::float(0.0).with_label("Minimum").with_range(-10.0, 10.0),
        "max" => Param::float(1.0).with_label("Maximum").with_range(-10.0, 10.0),
        "normscale" => Param::float(1.0).with_label("Norm Scale").with_range(0.0, 1.0),
        "step" => Param::float(0.1).with_label("Quantise Step").with_range(0.0, 10.0),
    }
}

fn apply_limit(v: f32, lo: f32, hi: f32, span: f32, step: f32, mode: usize) -> f32 {
    match mode {
        0 => v,
        1 => v.clamp(lo, hi),
        2 => lo + (v - lo).rem_euclid(span),
        3 => {
            let t = (v - lo).rem_euclid(span * 2.0);
            lo + if t <= span { t } else { span * 2.0 - t }
        }
        _ => {
            if step <= 0.0 {
                v
            } else {
                (v / step).round() * step
            }
        }
    }
}

fn normalise(channels: &mut [Channel], scale: f32, before: &[Channel]) {
    if scale <= 0.0 {
        return;
    }
    for (ch, orig) in channels.iter_mut().zip(before) {
        if ch.samples.len() <= 1 {
            continue;
        }
        let min = ch.samples.iter().copied().fold(f32::INFINITY, f32::min);
        let max = ch.samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        if (max - min).abs() < 1e-12 {
            continue;
        }
        for (v, &o) in ch.samples.iter_mut().zip(&orig.samples) {
            let t = (*v - min) / (max - min);
            let mapped = -1.0 + t * 2.0;
            *v = o + scale * (mapped - o);
        }
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let (lo, hi) = (c.f("min"), c.f("max"));
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let span = (hi - lo).max(1e-6);
    let step = c.f("step");
    let mode = c.menu("type");
    let norm = c.f("normscale");
    let mut data = map_samples(c.input(0), |_, _, v| apply_limit(v, lo, hi, span, step, mode));
    let before = data.channels.clone();
    normalise(&mut data.channels, norm, &before);
    data
}

pub const TYPE_NAME: &str = "limitCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Limit",
        &["in"],
        "Clamp, wrap, fold or quantise a channel.",
        params,
        cook,
    )
}