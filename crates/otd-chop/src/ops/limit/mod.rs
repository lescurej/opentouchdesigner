use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::ChopData;

use super::helpers::*;
use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("off", &["off", "clamp", "loop", "zigzag", "quantise"])
            .with_label("Type"),
        "min" => Param::float(0.0).with_label("Minimum").with_range(-10.0, 10.0),
        "max" => Param::float(1.0).with_label("Maximum").with_range(-10.0, 10.0),
        "norm" => Param::bool(false).with_label("Normalize"),
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

fn sample_span(samples: &[f32]) -> Option<(f32, f32)> {
    let mut lo = f32::INFINITY;
    let mut hi = f32::NEG_INFINITY;
    for v in samples {
        if v.is_finite() {
            lo = lo.min(*v);
            hi = hi.max(*v);
        }
    }
    if !lo.is_finite() || (hi - lo).abs() < 1e-12 {
        None
    } else {
        Some((lo, hi))
    }
}

fn stretch(samples: &mut [f32], in_lo: f32, in_hi: f32, out_lo: f32, out_hi: f32) {
    let span = in_hi - in_lo;
    for v in samples {
        let t = (*v - in_lo) / span;
        *v = out_lo + t * (out_hi - out_lo);
    }
}

fn running_span(state: &mut super::OpState, channel: usize, sample: f32) -> Option<(f32, f32)> {
    let lo_i = channel * 2;
    if state.i[channel] == 0 {
        state.f[lo_i] = sample;
        state.f[lo_i + 1] = sample;
        state.i[channel] = 1;
        return None;
    }
    state.f[lo_i] = state.f[lo_i].min(sample);
    state.f[lo_i + 1] = state.f[lo_i + 1].max(sample);
    sample_span(&[state.f[lo_i], state.f[lo_i + 1]])
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let (lo, hi) = (c.f("min"), c.f("max"));
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let span = (hi - lo).max(1e-6);
    let step = c.f("step");
    let mode = c.menu("type");
    let normalize = c.b("norm");
    if mode == 0 && !normalize {
        c.state.f.clear();
        c.state.i.clear();
        c.passthrough_input = Some(0);
        return ChopData::empty();
    }
    let mut data = map_samples(c.input(0), |_, _, v| apply_limit(v, lo, hi, span, step, mode));
    if !normalize {
        c.state.f.clear();
        c.state.i.clear();
        return data;
    }
    let n = data.channels.len();
    if c.state.i.len() != n {
        c.state.f = vec![0.0; n * 2];
        c.state.i = vec![0; n];
    }
    for (ci, ch) in data.channels.iter_mut().enumerate() {
        let range = if ch.as_slice().len() > 1 {
            sample_span(ch.as_slice())
        } else {
            ch.as_slice()
                .first()
                .copied()
                .filter(|v| v.is_finite())
                .and_then(|v| running_span(c.state, ci, v))
        };
        if let Some((in_lo, in_hi)) = range {
            stretch(ch.samples_mut(), in_lo, in_hi, lo, hi);
        }
    }
    data
}

pub const TYPE_NAME: &str = "limitCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Limit",
        &["in"],
        "Clamp, wrap, fold, quantise or normalise a channel.",
        params,
        cook,
    )
}