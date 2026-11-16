//! Elemental resample CHOPs (replacing the old monolithic Resample):
//! last value, trim a time range, set length, relabel sample rate.
//! Former *new interval*: Trim → Length (`duration × rate`) → Sample Rate.
//! To keep duration in seconds while changing rate: Length then Sample Rate.

use otd_core::Param;
use otd_core::indexmap::IndexMap;

use crate::data::{Channel, ChopData, MAX_SLICE};

use super::ChopCtx;
use super::helpers::{chop_common_page_params, merge_param_maps};

fn unit_to_seconds(val: f64, unit: usize, rate: f64, fps: f64) -> f64 {
    match unit {
        1 => val / fps.max(1e-6),
        2 => val / rate.max(1e-6),
        _ => val,
    }
}

fn interp_index(c: &ChopCtx) -> usize {
    let chosen = c
        .node
        .param("interp")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    match chosen.as_str() {
        "nearest" | "nointerp" => 0,
        "linear" => 1,
        "cubic" => 2,
        "edge" => 3,
        _ => c.menu("interp"),
    }
}

fn relative_mode(c: &ChopCtx) -> usize {
    let chosen = c
        .node
        .param("relative")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    match chosen.as_str() {
        "abs" | "Absolute" => 0,
        "rel" | "Relative to Start/End" => 1,
        "cur" | "Relative to Current Frame" => 2,
        _ => c.menu("relative"),
    }
}

fn lerp_resample(a: f32, b: f32, t: f32, correct: bool, cycle: f32) -> f32 {
    if !correct || !a.is_finite() || !b.is_finite() {
        return a + (b - a) * t;
    }
    let mut d = b - a;
    let half = cycle * 0.5;
    while d > half {
        d -= cycle;
    }
    while d < -half {
        d += cycle;
    }
    a + d * t
}

fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn sample_at(src: &[f32], pos: f32, interp: usize, correct: bool, cycle: f32) -> f32 {
    let n = src.len();
    if n == 0 {
        return 0.0;
    }
    if n == 1 {
        return src[0];
    }
    if interp == 0 {
        let i = pos.round() as usize;
        return src[i.min(n - 1)];
    }
    if interp == 2 {
        let i = pos.floor() as i32;
        let t = pos - i as f32;
        let i0 = (i - 1).max(0) as usize;
        let i1 = i.max(0) as usize;
        let i2 = (i + 1).min(n as i32 - 1) as usize;
        let i3 = (i + 2).min(n as i32 - 1) as usize;
        return catmull_rom(src[i0], src[i1], src[i2], src[i3], t);
    }
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(n - 1);
    let f = pos - lo as f32;
    if interp == 3 && (lo == hi || (f < 0.25 || f > 0.75)) {
        let pick = if f < 0.5 { lo } else { hi };
        return src[pick];
    }
    lerp_resample(src[lo], src[hi], f, correct, cycle)
}

fn resample_channel(
    src: &[f32],
    out_n: usize,
    interp: usize,
    correct: bool,
    cycle: f32,
) -> Vec<f32> {
    if out_n == 0 {
        return Vec::new();
    }
    if src.is_empty() {
        return vec![0.0; out_n];
    }
    if src.len() == 1 {
        return vec![src[0]; out_n];
    }
    (0..out_n)
        .map(|i| {
            let pos = i as f32 * (src.len() - 1) as f32 / (out_n.max(2) - 1) as f32;
            sample_at(src, pos, interp, correct, cycle)
        })
        .collect()
}

fn scale_constant_area(src: &[f32], samples: &mut [f32]) {
    let in_sum: f32 = src.iter().copied().sum();
    let out_sum: f32 = samples.iter().copied().sum();
    if out_sum.abs() > 1e-12 {
        let scale = in_sum / out_sum;
        for v in samples {
            *v *= scale;
        }
    }
}

fn time_span(c: &ChopCtx, src_len: usize, src_rate: f64) -> (f64, f64) {
    let dur = src_len as f64 / src_rate.max(1e-6);
    let mut start = unit_to_seconds(
        c.f("start") as f64,
        c.menu("startunit"),
        src_rate,
        c.time.fps,
    );
    let mut end = unit_to_seconds(c.f("end") as f64, c.menu("endunit"), src_rate, c.time.fps);
    match relative_mode(c) {
        1 => {
            start = start.clamp(0.0, dur);
            end = if end > 0.0 {
                end.clamp(start, dur)
            } else {
                (dur + end).clamp(start, dur)
            };
        }
        2 => {
            let t = c.time.time;
            start = (t + start).clamp(0.0, dur);
            end = (t + end).clamp(start, dur);
        }
        _ => {
            start = start.clamp(0.0, dur);
            end = end.clamp(start, dur);
        }
    }
    (start, end.max(start + 1e-9))
}

fn src_trim(src: &[f32], start_s: f64, end_s: f64, rate: f64) -> Vec<f32> {
    let n = src.len();
    if n == 0 {
        return vec![0.0];
    }
    if n == 1 {
        return vec![src[0]];
    }
    let i0 = (start_s * rate).floor() as usize;
    let i1 = ((end_s * rate).ceil() as usize).min(n).max(i0 + 1);
    src[i0..i1.min(n)].to_vec()
}

fn resample_channels<F>(c: &ChopCtx, input: &ChopData, out_n: usize, src_each: F) -> Vec<Channel>
where
    F: Fn(&[f32]) -> &[f32],
{
    let interp = interp_index(c);
    let correct = c.b("correct");
    let cycle = c.f("cyclelen").max(1e-6);
    let constarea = c.b("constarea");
    input
        .channels
        .iter()
        .map(|ch| {
            let src = src_each(&ch.samples);
            let mut samples = resample_channel(src, out_n, interp, correct, cycle);
            if constarea {
                scale_constant_area(src, &mut samples);
            }
            Channel::new(ch.name.clone(), samples)
        })
        .collect()
}

pub(crate) fn quality_params() -> IndexMap<String, Param> {
    params! {
        "interp" => Param::menu("linear", &["nointerp", "linear", "cubic", "edge"])
            .with_label("Interpolation"),
        "constarea" => Param::bool(false).with_label("Constant Area"),
        "correct" => Param::bool(false).with_label("Correct for Cycles"),
        "cyclelen" => Param::float(360.0)
            .with_label("Cycle Length")
            .with_range(0.01, 100_000.0),
    }
}

pub(crate) fn params_last() -> IndexMap<String, Param> {
    merge_param_maps(params! {}, chop_common_page_params())
}

/// One sample per channel: the most recent value in the buffer (or slice).
pub(crate) fn cook_last(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let channels: Vec<Channel> = input
        .channels
        .iter()
        .map(|ch| Channel::new(ch.name.clone(), vec![ch.last()]))
        .collect();
    let time_sliced = c.b("timeslice") || input.time_sliced;
    ChopData::new(channels, input.sample_rate, time_sliced)
}

pub(crate) fn params_length() -> IndexMap<String, Param> {
    let max_len = MAX_SLICE as f64;
    merge_param_maps(
        merge_param_maps(
            params! {
                "length" => Param::int(64)
                    .with_label("Length (samples)")
                    .with_range(1.0, max_len),
            },
            quality_params(),
        ),
        chop_common_page_params(),
    )
}

/// Change sample count; input sample rate is unchanged (former Resample *stretch*).
pub(crate) fn cook_length(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let out_n = c.i("length").clamp(1, MAX_SLICE as i64) as usize;
    let channels = resample_channels(c, &input, out_n, |s| s);
    let time_sliced = c.b("timeslice") || input.time_sliced;
    ChopData::new(channels, input.sample_rate, time_sliced)
}

pub(crate) fn params_samplerate() -> IndexMap<String, Param> {
    merge_param_maps(
        params! {
            "rate" => Param::float(crate::data::CONTROL_RATE)
                .with_label("Sample Rate")
                .with_range(1.0, 48000.0),
        },
        chop_common_page_params(),
    )
}

/// Same samples, new sample rate metadata (former Resample *index*).
pub(crate) fn cook_samplerate(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let src_rate = input.sample_rate.max(1e-6);
    let new_rate = c.f("rate") as f64;
    let new_rate = if new_rate > 0.0 { new_rate } else { src_rate }.max(1.0);
    let time_sliced = c.b("timeslice") || input.time_sliced;
    ChopData::new(input.channels.clone(), new_rate, time_sliced)
}

pub(crate) fn params_trim() -> IndexMap<String, Param> {
    merge_param_maps(
        params! {
            "relative" => Param::menu(
                "Absolute",
                &[
                    "Absolute",
                    "Relative to Start/End",
                    "Relative to Current Frame",
                ],
            )
            .with_label("Unit Values"),
            "start" => Param::float(0.0).with_label("Start"),
            "startunit" => Param::menu("seconds", &["seconds", "frames", "samples"])
                .with_label("Start Unit"),
            "end" => Param::float(1.0).with_label("End"),
            "endunit" => Param::menu("seconds", &["seconds", "frames", "samples"])
                .with_label("End Unit"),
        },
        chop_common_page_params(),
    )
}

/// Keep samples between start and end (seconds, frames or samples); rate unchanged.
pub(crate) fn cook_trim(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let src_rate = input.sample_rate.max(1e-6);
    let src_len = input.num_samples().max(1);
    let (win_start, win_end) = time_span(c, src_len, src_rate);
    let channels: Vec<Channel> = input
        .channels
        .iter()
        .map(|ch| {
            let samples = src_trim(&ch.samples, win_start, win_end, src_rate);
            Channel::new(ch.name.clone(), samples)
        })
        .collect();
    let time_sliced = c.b("timeslice") || input.time_sliced;
    ChopData::new(channels, src_rate, time_sliced)
}
