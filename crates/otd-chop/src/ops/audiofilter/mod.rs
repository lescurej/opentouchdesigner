//! Audio Filter CHOP — biquad IIR filters at audio rate (TouchDesigner-compatible).

use otd_core::Param;
use otd_core::indexmap::IndexMap;

use crate::data::Channel;
use crate::data::ChopData;
use crate::ops::ChopCtx;

use super::{spec_animated, ChopSpec};

pub const TYPE_NAME: &str = "audiofilterCHOP";
pub const AUDIO_FILTER: &str = TYPE_NAME;

const BUTTER_Q: [f32; 2] = [0.541_196_1, 1.306_562_9];
/// Biquad DF-II state magnitudes above this are clamped so a high-Q peak at a
/// very low cutoff cannot run away.
const STATE_LIMIT: f32 = 8.0;
/// At 48 kHz, cutoff below ~2.5 Hz is where high resonance becomes unstable in
/// single-precision cookbook biquads.
const MIN_CUTOFF_AT_48K: f32 = 2.5;

#[derive(Clone, Copy, Default)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Clone, Copy, Default)]
struct BiquadState {
    z1: f32,
    z2: f32,
}

/// Very low cutoffs leave IIR states near zero; subnormal floats can stall FP
/// units on some CPUs and look like a full cook freeze at 48 kHz.
fn flush_filter_sample(x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }
    if x.abs() < 1e-20 { 0.0 } else { x }
}

fn limit_state(s: &mut BiquadState) {
    s.z1 = flush_filter_sample(s.z1).clamp(-STATE_LIMIT, STATE_LIMIT);
    s.z2 = flush_filter_sample(s.z2).clamp(-STATE_LIMIT, STATE_LIMIT);
}

fn biquad_step(x: f32, c: &BiquadCoeffs, s: &mut BiquadState) -> f32 {
    let x = flush_filter_sample(x);
    limit_state(s);
    let y = flush_filter_sample(c.b0 * x + s.z1);
    if !y.is_finite() {
        *s = BiquadState::default();
        return 0.0;
    }
    s.z1 = flush_filter_sample(c.b1 * x - c.a1 * y + s.z2);
    s.z2 = flush_filter_sample(c.b2 * x - c.a2 * y);
    limit_state(s);
    y.clamp(-STATE_LIMIT, STATE_LIMIT)
}

fn clamp_fc(fc: f32, fs: f32) -> f32 {
    // Stay just under Nyquist. A CHOP's sample-rate slider floors at 1 Hz, and
    // `fs * 0.49` is then 0.49 — below the 1 Hz floor. `f32::clamp` panics
    // when its lower bound is above its upper bound.
    let hi = (fs * 0.49).max(f32::MIN_POSITIVE);
    let scaled_min = fs * (MIN_CUTOFF_AT_48K / 48_000.0);
    let lo = 1.0_f32.max(scaled_min).min(hi);
    if fc.is_finite() { fc.clamp(lo, hi) } else { lo }
}

/// High resonance at very low cutoffs drives poles toward the unit circle in f32.
fn resonance_scale(fc: f32, fs: f32) -> f32 {
    let w0 = std::f32::consts::TAU * fc.max(f32::MIN_POSITIVE) / fs.max(1.0);
    // Full resonance from ~10 Hz upward at 48 kHz; taper below that.
    (w0 / 0.0013).clamp(0.0, 1.0)
}

fn q_from_resonance(resonance: f32, base: f32, fc: f32, fs: f32) -> f32 {
    let r = resonance.max(0.0) * resonance_scale(fc, fs);
    (base * (1.0 + r * 0.35)).clamp(0.1, 50.0)
}

fn coeffs_finite(c: &BiquadCoeffs) -> bool {
    c.b0.is_finite() && c.b1.is_finite() && c.b2.is_finite() && c.a1.is_finite() && c.a2.is_finite()
}

/// Normalized denominator `1 + a1 z^-1 + a2 z^-2`: poles inside the unit circle.
fn coeffs_stable(c: &BiquadCoeffs) -> bool {
    if !coeffs_finite(c) {
        return false;
    }
    let a1 = c.a1;
    let a2 = c.a2;
    (1.0 + a1 + a2) > 0.0 && (1.0 - a1 + a2) > 0.0 && (1.0 - a2) > 0.0
}

fn passthrough_coeffs(c: &mut BiquadCoeffs) {
    *c = BiquadCoeffs {
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
        a1: 0.0,
        a2: 0.0,
    };
}

fn fill_coeffs_checked(kind: usize, c: &mut BiquadCoeffs, fs: f32, fc: f32, q: f32) {
    fill_coeffs(kind, c, fs, fc, q);
    if !coeffs_stable(c) {
        passthrough_coeffs(c);
    }
}

fn fill_lowpass(c: &mut BiquadCoeffs, fs: f32, fc: f32, q: f32) {
    let w0 = std::f32::consts::TAU * clamp_fc(fc, fs) / fs;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = (1.0 - cos_w0) / 2.0;
    let b1 = 1.0 - cos_w0;
    let b2 = b0;
    let a0 = 1.0 + alpha;
    *c = BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: (-2.0 * cos_w0) / a0,
        a2: (1.0 - alpha) / a0,
    };
}

fn fill_highpass(c: &mut BiquadCoeffs, fs: f32, fc: f32, q: f32) {
    let w0 = std::f32::consts::TAU * clamp_fc(fc, fs) / fs;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = (1.0 + cos_w0) / 2.0;
    let b1 = -(1.0 + cos_w0);
    let b2 = b0;
    let a0 = 1.0 + alpha;
    *c = BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: (-2.0 * cos_w0) / a0,
        a2: (1.0 - alpha) / a0,
    };
}

fn fill_bandpass(c: &mut BiquadCoeffs, fs: f32, fc: f32, q: f32) {
    let w0 = std::f32::consts::TAU * clamp_fc(fc, fs) / fs;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
    let a0 = 1.0 + alpha;
    *c = BiquadCoeffs {
        b0: b0 / a0,
        b1,
        b2: b2 / a0,
        a1: (-2.0 * cos_w0) / a0,
        a2: (1.0 - alpha) / a0,
    };
}

fn fill_notch(c: &mut BiquadCoeffs, fs: f32, fc: f32, q: f32) {
    let w0 = std::f32::consts::TAU * clamp_fc(fc, fs) / fs;
    let cos_w0 = w0.cos();
    let sin_w0 = w0.sin();
    let alpha = sin_w0 / (2.0 * q);
    let b0 = 1.0;
    let b1 = -2.0 * cos_w0;
    let b2 = 1.0;
    let a0 = 1.0 + alpha;
    *c = BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: (-2.0 * cos_w0) / a0,
        a2: (1.0 - alpha) / a0,
    };
}

fn fill_coeffs(kind: usize, c: &mut BiquadCoeffs, fs: f32, fc: f32, q: f32) {
    match kind {
        1 => fill_highpass(c, fs, fc, q),
        2 => fill_bandpass(c, fs, fc, q),
        3 => fill_notch(c, fs, fc, q),
        _ => fill_lowpass(c, fs, fc, q),
    }
}

fn cutoff_hz(c: &ChopCtx, fs: f32, modulation: Option<f32>) -> f32 {
    let raw = if let Some(hz) = modulation {
        hz
    } else if c.menu("units") == 0 {
        10f32.powf(c.f("cutofflog"))
    } else {
        c.f("cutofffrequency")
    };
    clamp_fc(raw, fs)
}

pub(crate) fn params() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "filter".into(),
        Param::menu(
            "lowpass",
            &["lowpass", "highpass", "bandpass", "bandreject"],
        )
        .with_label("Filter"),
    );
    m.insert(
        "units".into(),
        Param::menu("logarithmic", &["logarithmic", "frequency"]).with_label("Cutoff Units"),
    );
    m.insert(
        "cutofflog".into(),
        Param::float(3.0)
            .with_label("Filter Cutoff (Hz = 10**val)")
            .with_range(0.0, 4.5),
    );
    m.insert(
        "cutofffrequency".into(),
        Param::float(1000.0)
            .with_label("Filter Cutoff (Hz)")
            .with_range(1.0, 22000.0)
            .with_logarithmic(20.0),
    );
    m.insert(
        "resonance".into(),
        Param::float(0.0)
            .with_label("Filter Resonance")
            .with_range(0.0, 10.0),
    );
    m.insert(
        "rolloff".into(),
        Param::menu("24", &["12", "24"]).with_label("Roll-Off (dB per Octave)"),
    );
    m.insert(
        "drywet".into(),
        Param::float(1.0)
            .with_label("Dry / Wet Mix")
            .with_range(0.0, 1.0),
    );
    m
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    if input.num_channels() == 0 {
        return input;
    }

    let kind = c.menu("filter");
    let resonance = c.f("resonance").max(0.0);
    let steep = c.menu("rolloff") == 1;
    let drywet = c.f("drywet").clamp(0.0, 1.0);
    let fs = input.sample_rate.max(1.0) as f32;
    let mod_in = c.input(1);
    let mod_samples: Option<Vec<f32>> = if mod_in.num_channels() == 0 {
        None
    } else if let Some(ch) = mod_in.channel("cutofffrequency") {
        Some(ch.samples.clone())
    } else {
        mod_in.channels.first().map(|ch| ch.samples.clone())
    };
    let base_fc = cutoff_hz(c, fs, None);

    let n_ch = input.num_channels();
    // Four state floats per channel — two biquad stages (DF-II).
    c.state.fit(n_ch * 4);

    let mut channels = Vec::with_capacity(n_ch);
    for (ci, ch) in input.channels.iter().enumerate() {
        let base = ci * 4;
        let mut s0 = BiquadState {
            z1: c.state.f[base],
            z2: c.state.f[base + 1],
        };
        let mut s1 = BiquadState {
            z1: c.state.f[base + 2],
            z2: c.state.f[base + 3],
        };

        let mut c0 = BiquadCoeffs::default();
        let mut c1 = BiquadCoeffs::default();
        let mut last_fc = -1.0f32;

        let mut out = Vec::with_capacity(ch.samples.len());
        for (si, &dry) in ch.samples.iter().enumerate() {
            if !dry.is_finite() {
                s0 = BiquadState::default();
                s1 = BiquadState::default();
                out.push(0.0);
                continue;
            }
            let fc = if let Some(mod_slice) = mod_samples.as_deref() {
                let v = mod_slice
                    .get(si)
                    .copied()
                    .or_else(|| mod_slice.last().copied())
                    .unwrap_or(base_fc);
                cutoff_hz(c, fs, Some(v))
            } else {
                base_fc
            };

            if (fc - last_fc).abs() > 0.5 || si == 0 {
                let q0 = if steep {
                    q_from_resonance(resonance, BUTTER_Q[0], fc, fs)
                } else {
                    q_from_resonance(resonance, 0.707, fc, fs)
                };
                fill_coeffs_checked(kind, &mut c0, fs, fc, q0);
                if steep {
                    let q1 = q_from_resonance(resonance, BUTTER_Q[1], fc, fs);
                    fill_coeffs_checked(kind, &mut c1, fs, fc, q1);
                } else {
                    passthrough_coeffs(&mut c1);
                }
                if last_fc > 0.0 && (fc - last_fc).abs() > fc * 0.25 + 1.0 {
                    // Large cutoff moves with fragile poles: restart DF-II state.
                    s0 = BiquadState::default();
                    s1 = BiquadState::default();
                }
                last_fc = fc;
            }

            let mut wet = biquad_step(dry, &c0, &mut s0);
            if steep {
                wet = biquad_step(wet, &c1, &mut s1);
            }
            out.push(dry * (1.0 - drywet) + wet * drywet);
        }

        c.state.f[base] = s0.z1;
        c.state.f[base + 1] = s0.z2;
        c.state.f[base + 2] = s1.z1;
        c.state.f[base + 3] = s1.z2;
        channels.push(Channel::new(ch.name.clone(), out));
    }

    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Audio Filter",
        &["in", "cutoff"],
        "Low-pass, high-pass, band-pass or band-reject IIR filter; 12 or 24 dB/octave.",
        params,
        cook,
    )
}

#[cfg(test)]
#[path = "../../tests/audiofilter__tests.rs"]
mod tests;
