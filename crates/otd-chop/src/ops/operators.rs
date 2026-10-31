use otd_core::indexmap::IndexMap;
use otd_core::{Crossing, EvalContext, Param};

use crate::data::{CONTROL_RATE, Channel, ChopData, slice_len, slice_times};

use super::helpers::*;
use super::ChopCtx;

// ------------------------------------------------------------- constant

pub const CONSTANT: &str = "constantCHOP";
pub const CONSTANT_MAX_ROWS: i64 = 64;

pub(crate) fn params_constant() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "rows".into(),
        Param::int(1)
            .with_label("Rows")
            .with_page("Constant")
            .with_range(1.0, CONSTANT_MAX_ROWS as f64),
    );
    for i in 0..CONSTANT_MAX_ROWS as usize {
        let default_name = if i == 0 { "chan1" } else { "chan" };
        m.insert(
            format!("name{i}"),
            Param::str(default_name)
                .with_label("Channel Name")
                .with_page("Constant"),
        );
        m.insert(
            format!("value{i}"),
            Param::float(0.0)
                .with_label("Value")
                .with_page("Constant"),
        );
    }
    with_rate(m)
}

pub(crate) fn cook_constant(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let channels = if c.node.param("rows").is_some() {
        cook_constant_rows(c, n)
    } else {
        cook_constant_legacy(c, n)
    };
    ChopData::new(channels, rate, true)
}

pub(crate) fn cook_constant_rows(c: &ChopCtx<'_>, n: usize) -> Vec<Channel> {
    let rows = c.i("rows").clamp(1, CONSTANT_MAX_ROWS) as usize;
    let mut channels = Vec::new();
    for i in 0..rows {
        let name_tpl = c.s(&format!("name{i}"));
        let val = c.f(&format!("value{i}"));
        let count = otd_core::channel_count_from_name(&name_tpl);
        let names = otd_core::expand_channel_names(&name_tpl, count);
        for name in names {
            channels.push(Channel::constant(name, val, n));
        }
    }
    channels
}

pub(crate) fn cook_constant_legacy(c: &ChopCtx<'_>, n: usize) -> Vec<Channel> {
    let name = c.s("name");
    let count = otd_core::channel_count_from_name(&name);
    let names = otd_core::expand_channel_names(&name, count);
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| Channel::constant(name, c.f(&format!("value{i}")), n))
        .collect()
}

// ----------------------------------------------------------- parameter

pub const PARAMETER: &str = "parameterCHOP";
pub const INFO: &str = "infoCHOP";

pub(crate) fn params_info() -> IndexMap<String, Param> {
    params! {
        "op" => Param::str("..").with_label("Operator").as_path_ref(),
    }
}

pub(crate) fn cook_info(c: &mut ChopCtx) -> ChopData {
    let Some(info) = c.foreign.first().and_then(|x| x.as_ref()) else {
        return ChopData::empty();
    };
    let Some(pairs) = info.as_info() else {
        return ChopData::empty();
    };
    c.state.fit(pairs.len());
    let channels: Vec<Channel> = pairs
        .iter()
        .enumerate()
        .map(|(ci, (name, value))| {
            let end = *value;
            c.state.prev[ci] = end;
            Channel::new(name.clone(), vec![end])
        })
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}

pub(crate) fn params_parameter() -> IndexMap<String, Param> {
    params! {
        "op" => Param::str("..").with_label("Operator").as_path_ref(),
        "parameters" => Param::str("value").with_label("Parameters"),
        "name" => Param::str("v").with_label("Channel Name"),
    }
}

fn resolve_op_path(c: &ChopCtx<'_>, path: &str) -> Option<otd_core::NodeId> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        c.graph.find(path)
    } else {
        c.graph.find_from(c.id, path)
    }
}

pub(crate) fn cook_parameter(c: &mut ChopCtx) -> ChopData {
    let watch = match resolve_op_path(c, &c.s("op")) {
        Some(id) => id,
        None => return ChopData::empty(),
    };
    let watch_path = c.graph.path(watch);
    let prefix = c.s("name");
    let names: Vec<String> = c
        .s("parameters")
        .split_whitespace()
        .filter(|n| !n.is_empty() && *n != "*")
        .map(|s| s.to_string())
        .collect();
    let chan_names = otd_core::expand_channel_names(&prefix, names.len());
    let channels: Vec<Channel> = names
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let value = c
                .graph
                .get(watch)
                .and_then(|n| n.param(key))
                .map(|p| {
                    p.eval(&EvalContext {
                        channels: c.eval.channels,
                        path: Some(&watch_path),
                        ..*c.eval
                    })
                    .as_f32()
                })
                .unwrap_or(0.0);
            let chan = chan_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("{prefix}{}", i + 1));
            Channel::constant(chan, value, 1)
        })
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}

// ------------------------------------------------------------------ LFO

pub(crate) fn params_lfo() -> IndexMap<String, Param> {
    with_rate(params! {
        "type" => Param::menu("sine", &["sine", "triangle", "ramp", "square", "pulse"])
            .with_label("Type"),
        "frequency" => Param::float(1.0).with_label("Frequency").with_range(0.0, 20.0),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-2.0, 2.0),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "pulsewidth" => Param::float(0.5).with_label("Pulse Width").with_range(0.0, 1.0),
        "name" => Param::str("lfo").with_label("Channel Name"),
    })
}

fn wave(kind: usize, phase: f32, pulse_width: f32) -> f32 {
    let p = phase - phase.floor();
    match kind {
        // triangle
        1 => 1.0 - 4.0 * (p - 0.5).abs(),
        // ramp
        2 => p * 2.0 - 1.0,
        // square
        3 => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        // pulse
        4 => {
            if p < pulse_width.clamp(0.0, 1.0) {
                1.0
            } else {
                -1.0
            }
        }
        _ => (p * std::f32::consts::TAU).sin(),
    }
}

pub(crate) fn cook_lfo(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let (kind, freq, amp, offset, phase, pw) = (
        c.menu("type"),
        c.f("frequency"),
        c.f("amplitude"),
        c.f("offset"),
        c.f("phase"),
        c.f("pulsewidth"),
    );
    let samples: Vec<f32> = slice_times(c.time, n)
        .map(|t| wave(kind, t as f32 * freq + phase, pw) * amp + offset)
        .collect();
    let name_tpl = c.s("name");
    let count = otd_core::channel_count_from_name(&name_tpl);
    let names = otd_core::expand_channel_names(&name_tpl, count);
    let channels: Vec<Channel> = names
        .into_iter()
        .map(|name| Channel::new(name, samples.clone()))
        .collect();
    ChopData::new(channels, rate, true)
}

// ------------------------------------------------------------ animation

/// The starting keys in a new Animation CHOP: a one-second ramp, so the
/// operator animates something the moment it is created.
const DEFAULT_KEYS: &str = "\
# channel  time  value  interpolation
chan1      0     0      smooth
chan1      1     1      smooth
";

pub(crate) fn params_animation() -> IndexMap<String, Param> {
    with_rate(params! {
        "keys" => Param::str(DEFAULT_KEYS).with_label("Keys"),
        "play" => Param::menu("timeline", &["timeline", "loop", "hold"]).with_label("Play"),
        "speed" => Param::float(1.0).with_label("Speed").with_range(-4.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-60.0, 60.0),
    })
}

pub(crate) fn cook_animation(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let (curves, _) = crate::anim::Curves::parse(&c.s("keys"));

    let mode = c.menu("play");
    let (speed, offset) = (c.f("speed") as f64, c.f("offset") as f64);
    // `loop` needs the keyed span; without keys there is nothing to wrap into
    // and the curve is flat anyway.
    let span = curves.range().filter(|(lo, hi)| hi > lo);

    let at = |t: f64| -> f64 {
        let t = t * speed + offset;
        match (mode, span) {
            // 1 == loop. Wrapping with rem_euclid rather than `%` so it works
            // going backwards too, which a negative speed does.
            (1, Some((lo, hi))) => lo + (t - lo).rem_euclid(hi - lo),
            _ => t,
        }
    };

    // One sample per slice sample, so an exported parameter reads the value at
    // *this* instant and a downstream filter sees a continuous signal — the
    // same time-slicing contract every other generator here keeps.
    let times: Vec<f64> = slice_times(c.time, n).map(at).collect();
    let channels = curves
        .0
        .iter()
        .map(|(name, curve)| {
            Channel::new(
                name.clone(),
                times.iter().map(|t| curve.sample(*t)).collect(),
            )
        })
        .collect::<Vec<_>>();

    // A CHOP with no channels at all is awkward downstream; an animation with
    // no keys yet reports one silent channel rather than nothing.
    let channels = if channels.is_empty() {
        vec![Channel::new("chan1".to_string(), vec![0.0; n])]
    } else {
        channels
    };
    ChopData::new(channels, rate, true)
}

// ---------------------------------------------------------------- noise

pub(crate) fn params_noise() -> IndexMap<String, Param> {
    with_rate(params! {
        "type" => Param::menu("sparse", &["sparse", "hermite", "harmonic", "brownian", "random", "alligator"]).with_label("Type"),
        "channelname" => Param::str("chan1").with_label("Channel Names"),
        "start" => Param::float(0.0).with_label("Start"),
        "startunit" => Param::menu("seconds", &["seconds", "frames", "samples"]).with_label("Start Unit"),
        "end" => Param::float(10.0).with_label("End"),
        "endunit" => Param::menu("seconds", &["seconds", "frames", "samples"]).with_label("End Unit"),
        "timeslice" => Param::bool(false).with_label("Time Slice"),
        "period" => Param::float(1.0).with_label("Period").with_range(0.0, 10.0),
        "periodunit" => Param::menu("seconds", &["seconds", "frames", "samples"]).with_label("Period Unit"),
        "harmon" => Param::int(0).with_label("Harmonics").with_range(0.0, 20.0),
        "spread" => Param::float(2.0).with_label("Harmonic Spread").with_range(1.0, 4.0),
        "rough" => Param::float(0.5).with_label("Roughness").with_range(0.0, 1.0),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-2.0, 2.0),
        "seed" => Param::float(0.0).with_label("Seed"),
    })
}

fn unit_to_seconds(val: f64, unit: usize, rate: f64, fps: f64) -> f64 {
    match unit {
        1 => val / fps.max(1e-6),
        2 => val / rate.max(1e-6),
        _ => val,
    }
}

fn noise_channel_field(c: &ChopCtx) -> String {
    let cn = c.s("channelname");
    if !cn.trim().is_empty() {
        return cn;
    }
    let legacy = c.s("name");
    if !legacy.trim().is_empty() {
        return legacy;
    }
    "chan1".into()
}

fn noise_type_index(c: &ChopCtx) -> usize {
    let chosen = c
        .node
        .param("type")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    match chosen.as_str() {
        "smooth" => 0,
        "sparse" => 0,
        "hermite" => 1,
        "harmonic" | "harmonic summation" => 2,
        "brownian" => 3,
        "random" => 4,
        "alligator" => 5,
        _ => c.menu("type"),
    }
}

fn noise_period_seconds(c: &ChopCtx, rate: f64) -> f32 {
    let p = c.f("period");
    if noise_type_index(c) == 4 && p <= 0.0 {
        return 0.0;
    }
    let p = p.max(1e-6);
    match c.menu("periodunit") {
        1 => (p as f64 / c.time.fps.max(1e-6)) as f32,
        2 => (p as f64 / rate.max(1e-6)) as f32,
        _ => p,
    }
}

fn noise_sample_count(c: &ChopCtx, rate: f64) -> usize {
    if c.node.param("start").is_some() {
        let start = unit_to_seconds(c.f("start") as f64, c.menu("startunit"), rate, c.time.fps);
        let end = unit_to_seconds(c.f("end") as f64, c.menu("endunit"), rate, c.time.fps);
        let secs = (end - start).max(0.0);
        ((secs * rate).round() as usize).clamp(1, 8192)
    } else {
        c.i("length").clamp(1, 8192) as usize
    }
}

fn noise_start_seconds(c: &ChopCtx) -> f64 {
    let rate = c.rate();
    if c.node.param("start").is_some() {
        unit_to_seconds(c.f("start") as f64, c.menu("startunit"), rate, c.time.fps)
    } else {
        0.0
    }
}

fn hash1(x: i64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9e3779b97f4a7c15);
    h ^= h >> 29;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 32;
    (h as u32 as f64 / u32::MAX as f64) as f32 * 2.0 - 1.0
}

fn value_noise(t: f32, seed: i64) -> f32 {
    let i = t.floor();
    let f = t - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash1(i as i64 + seed * 7919);
    let b = hash1(i as i64 + 1 + seed * 7919);
    a + (b - a) * u
}

fn hermite_noise(t: f32, seed: i64) -> f32 {
    let i = t.floor() as i64;
    hash1(i + seed * 7919)
}

fn noise_octaves(
    t: f32,
    seed: i64,
    harmonics: i32,
    rough: f32,
    spread: f32,
    sample: fn(f32, i64) -> f32,
) -> f32 {
    let hmax = harmonics.max(0);
    if hmax == 0 {
        return sample(t, seed);
    }
    let mut amp = 1.0f32;
    let mut freq = 1.0f32;
    let mut sum = 0.0f32;
    let mut norm = 0.0f32;
    for h in 0..=hmax {
        sum += amp * sample(t * freq, seed + h as i64 * 997);
        norm += amp;
        amp *= rough.clamp(0.0, 1.0);
        freq *= spread.max(1.0);
    }
    if norm > 0.0 {
        sum / norm
    } else {
        sample(t, seed)
    }
}

fn alligator_noise(t: f32, seed: i64) -> f32 {
    hash1(t.floor() as i64 + seed * 7919)
}

fn noise_sample(
    t_sec: f64,
    period_sec: f32,
    seed: i64,
    kind: usize,
    harmonics: i32,
    rough: f32,
    spread: f32,
    white: &mut u64,
) -> f32 {
    if kind == 4 {
        if period_sec <= 0.0 {
            *white = white.wrapping_mul(6364136223846793005).wrapping_add(1);
            let h = (*white >> 32) as i64 ^ seed;
            return hash1(h);
        }
        let x = (t_sec as f32) / period_sec;
        return hash1((x * 1_000_000.0) as i64 + seed);
    }
    if period_sec <= 0.0 {
        return 0.0;
    }
    let x = (t_sec as f32) / period_sec;
    match kind {
        1 => noise_octaves(x, seed, harmonics, rough, spread, hermite_noise),
        2 => noise_octaves(x, seed, harmonics, rough, spread, value_noise),
        3 => noise_octaves(x, seed, harmonics, rough, spread, value_noise),
        5 => alligator_noise(x, seed),
        _ => noise_octaves(x, seed, harmonics, rough, spread, value_noise),
    }
}

pub(crate) fn cook_noise(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let time_slice = c.b("timeslice");
    let n = if time_slice {
        slice_len(rate, c.time)
    } else {
        noise_sample_count(c, rate)
    };
    let names = otd_core::expand_channel_name_field(&noise_channel_field(c));
    c.state.fit(names.len());
    let kind = noise_type_index(c);
    let period_sec = noise_period_seconds(c, rate);
    let amp = c.f("amplitude");
    let offset = c.f("offset");
    let base_seed = c.f("seed") as i64;
    let harmonics = c.i("harmon") as i32;
    let rough = c.f("rough");
    let spread = c.f("spread");
    let t0 = if time_slice {
        c.time.time - c.time.dt
    } else {
        noise_start_seconds(c)
    };

    let channels = names
        .into_iter()
        .enumerate()
        .map(|(ch, name)| {
            let seed = otd_core::channel_seed_for_name(base_seed, &name, ch);
            let mut brown = 0.0f32;
            let mut white = seed as u64 ^ 0x9e3779b97f4a7c15;
            let samples: Vec<f32> = (0..n)
                .map(|i| {
                    let t = if time_slice {
                        t0 + (i + 1) as f64 * c.time.dt / n.max(1) as f64
                    } else {
                        t0 + i as f64 / rate.max(1e-6)
                    };
                    let mut v = noise_sample(
                        t,
                        period_sec,
                        seed,
                        kind,
                        harmonics,
                        rough,
                        spread,
                        &mut white,
                    );
                    if kind == 3 {
                        brown += v * (1.0 / rate.max(1.0) as f32);
                        v = brown;
                    }
                    v * amp + offset
                })
                .collect();
            Channel::new(name, samples)
        })
        .collect();
    ChopData::new(channels, rate, time_slice)
}

// -------------------------------------------------------------- pattern

pub(crate) fn params_pattern() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("ramp", &["ramp", "sine", "triangle", "square", "gaussian"])
            .with_label("Type"),
        "length" => Param::int(64).with_label("Length").with_range(1.0, 4096.0),
        "periods" => Param::float(1.0).with_label("Periods").with_range(0.1, 32.0),
        "amplitude" => Param::float(1.0).with_label("Amplitude").with_range(0.0, 4.0),
        "offset" => Param::float(0.0).with_label("Offset").with_range(-2.0, 2.0),
        "phase" => Param::float(0.0).with_label("Phase").with_range(0.0, 1.0),
        "name" => Param::str("chan1").with_label("Channel Name"),
    }
}

pub(crate) fn cook_pattern(c: &mut ChopCtx) -> ChopData {
    let len = c.i("length").clamp(1, 4096) as usize;
    let (kind, periods, amp, offset, phase) = (
        c.menu("type"),
        c.f("periods"),
        c.f("amplitude"),
        c.f("offset"),
        c.f("phase"),
    );
    let samples: Vec<f32> = (0..len)
        .map(|i| {
            let x = i as f32 / len as f32;
            let p = x * periods + phase;
            let v = match kind {
                0 => p - p.floor(),
                4 => {
                    let g = (x - 0.5) * 6.0;
                    (-0.5 * g * g).exp()
                }
                other => wave(other, p, 0.5),
            };
            v * amp + offset
        })
        .collect();
    let name_tpl = c.s("name");
    let count = otd_core::channel_count_from_name(&name_tpl);
    let names = otd_core::expand_channel_names(&name_tpl, count);
    let channels: Vec<Channel> = names
        .into_iter()
        .map(|name| Channel::new(name, samples.clone()))
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}

// ----------------------------------------------------------------- math

const MATH_UNARY: &[&str] = &["off", "negate", "pos", "root", "square", "inverse"];
const MATH_COMBINE: &[&str] = &[
    "off",
    "add",
    "subtract",
    "multiply",
    "divide",
    "average",
    "minimum",
    "maximum",
    "length",
];
const MATH_ALIGN: &[&str] = &[
    "auto",
    "none",
    "stretch",
    "start",
    "end",
    "shift1",
    "trim1",
    "stretch1",
    "trim",
    "squash",
];

pub(crate) fn params_math() -> IndexMap<String, Param> {
    params! {
        "preop" => Param::menu("off", MATH_UNARY)
            .with_label("Channel Pre OP")
            .with_page("Math"),
        "chanop" => Param::menu("off", MATH_COMBINE)
            .with_label("Combine Channels")
            .with_page("Math"),
        "combine" => Param::menu("add", MATH_COMBINE)
            .with_label("Combine CHOPs")
            .with_page("Math"),
        "postop" => Param::menu("off", MATH_UNARY)
            .with_label("Channel Post OP")
            .with_page("Math"),
        "match" => Param::menu("index", &["index", "name"])
            .with_label("Match By")
            .with_page("Math"),
        "align" => Param::menu("auto", MATH_ALIGN)
            .with_label("Align")
            .with_page("Math"),
        "integer" => Param::menu("off", &["off", "ceiling", "floor", "round"])
            .with_label("Integer")
            .with_page("Math"),
        "preoff" => Param::float(0.0)
            .with_label("Pre-Add")
            .with_range(-4.0, 4.0)
            .with_page("Mult-Add"),
        "gain" => Param::float(1.0)
            .with_label("Multiply")
            .with_range(-4.0, 4.0)
            .with_page("Mult-Add"),
        "offset" => Param::float(0.0)
            .with_label("Post-Add")
            .with_range(-4.0, 4.0)
            .with_page("Mult-Add"),
        "fromrange1" => Param::float(0.0)
            .with_label("From Range Min")
            .with_range(-1e4, 1e4)
            .with_page("Range"),
        "fromrange2" => Param::float(1.0)
            .with_label("From Range Max")
            .with_range(-1e4, 1e4)
            .with_page("Range"),
        "torange1" => Param::float(0.0)
            .with_label("To Range Min")
            .with_range(-1e4, 1e4)
            .with_page("Range"),
        "torange2" => Param::float(1.0)
            .with_label("To Range Max")
            .with_range(-1e4, 1e4)
            .with_page("Range"),
        "clamp" => Param::bool(false).with_label("Clamp").with_page("Range"),
        "clampmin" => Param::float(0.0).with_label("Clamp Min").with_page("Range"),
        "clampmax" => Param::float(1.0).with_label("Clamp Max").with_page("Range"),
        "scope" => Param::str("*").with_label("Scope").with_page("Common"),
    }
}

fn math_unary(op: usize, v: f32) -> f32 {
    match op {
        1 => -v,
        2 => v.abs(),
        3 => v.max(0.0).sqrt(),
        4 => v * v,
        5 => {
            if v == 0.0 {
                0.0
            } else {
                1.0 / v
            }
        }
        _ => v,
    }
}

fn math_sample_at(ch: &Channel, i: usize) -> f32 {
    if ch.samples.is_empty() {
        0.0
    } else {
        ch.samples[i.min(ch.samples.len() - 1)]
    }
}

fn math_reduce_combine(op: usize, vals: &[f32]) -> f32 {
    if vals.is_empty() {
        return 0.0;
    }
    match op {
        1 => vals.iter().sum(),
        2 => vals.iter().skip(1).fold(vals[0], |a, b| a - b),
        3 => vals.iter().product(),
        4 => vals.iter().skip(1).fold(vals[0], |a, b| {
            if *b == 0.0 {
                0.0
            } else {
                a / b
            }
        }),
        5 => vals.iter().sum::<f32>() / vals.len() as f32,
        6 => vals.iter().copied().fold(f32::INFINITY, f32::min),
        7 => vals.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        8 => vals.iter().map(|v| v * v).sum::<f32>().sqrt(),
        _ => vals[0],
    }
}

fn math_combine_pair(op: usize, a: f32, b: f32) -> f32 {
    math_reduce_combine(op, &[a, b])
}

fn math_combine_identity(op: usize) -> f32 {
    match op {
        3 => 1.0,
        4 => 1.0,
        6 => f32::INFINITY,
        7 => f32::NEG_INFINITY,
        8 => 0.0,
        _ => 0.0,
    }
}

fn math_prepare_input(data: ChopData, preop: usize, chanop: usize) -> ChopData {
    if data.channels.is_empty() {
        return data;
    }
    let len = data.num_samples().max(1);
    let mut channels: Vec<Channel> = data
        .channels
        .into_iter()
        .map(|mut ch| {
            for s in &mut ch.samples {
                *s = math_unary(preop, *s);
            }
            ch
        })
        .collect();
    if chanop != 0 && channels.len() > 1 {
        let name = channels[0].name.clone();
        let samples: Vec<f32> = (0..len)
            .map(|si| {
                let vals: Vec<f32> = channels.iter().map(|ch| math_sample_at(ch, si)).collect();
                math_reduce_combine(chanop, &vals)
            })
            .collect();
        channels = vec![Channel::new(name, samples)];
    }
    ChopData::new(channels, data.sample_rate, data.time_sliced)
}

fn math_align_inputs(mut a: ChopData, mut b: ChopData, align: usize) -> (ChopData, ChopData) {
    let la = a.num_samples();
    let lb = b.num_samples();
    let trim = matches!(align, 6 | 8);
    let len = if trim {
        la.min(lb).max(1)
    } else {
        la.max(lb).max(1)
    };
    if trim {
        for ch in &mut a.channels {
            ch.samples.truncate(len);
        }
        for ch in &mut b.channels {
            ch.samples.truncate(len);
        }
    } else {
        a.pad_to(len);
        b.pad_to(len);
    }
    (a, b)
}

fn math_pick_channel(data: &ChopData, ci: usize) -> Option<Channel> {
    if data.num_channels() == 1 {
        data.nth(0).cloned()
    } else {
        data.nth(ci).cloned()
    }
}

fn math_combine_chops_by_index(a: &ChopData, b: &ChopData, chopop: usize, len: usize) -> Vec<Channel> {
    let channel_count = a.num_channels().max(b.num_channels()).max(1);
    let identity = math_combine_identity(chopop);
    (0..channel_count)
        .map(|ci| {
            let ca = math_pick_channel(a, ci);
            let cb = math_pick_channel(b, ci);
            let name = ca
                .as_ref()
                .or(cb.as_ref())
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("chan{}", ci + 1));
            let samples: Vec<f32> = (0..len)
                .map(|si| {
                    let va = ca
                        .as_ref()
                        .map(|c| math_sample_at(c, si))
                        .unwrap_or(identity);
                    let vb = cb
                        .as_ref()
                        .map(|c| math_sample_at(c, si))
                        .unwrap_or(identity);
                    if ca.is_none() {
                        vb
                    } else if cb.is_none() {
                        va
                    } else {
                        math_combine_pair(chopop, va, vb)
                    }
                })
                .collect();
            Channel::new(name, samples)
        })
        .collect()
}

fn math_combine_chops_by_name(a: &ChopData, b: &ChopData, chopop: usize, len: usize) -> Vec<Channel> {
    let mut out = Vec::new();
    let mut used_b = vec![false; b.num_channels()];
    for ca in &a.channels {
        if let Some((bi, cb)) = b
            .channels
            .iter()
            .enumerate()
            .find(|(_, c)| c.name == ca.name)
        {
            used_b[bi] = true;
            let samples: Vec<f32> = (0..len)
                .map(|si| math_combine_pair(chopop, math_sample_at(ca, si), math_sample_at(cb, si)))
                .collect();
            out.push(Channel::new(ca.name.clone(), samples));
        } else {
            out.push(ca.clone());
        }
    }
    for (bi, cb) in b.channels.iter().enumerate() {
        if !used_b[bi] {
            out.push(cb.clone());
        }
    }
    out
}

fn math_finish_sample(
    v: f32,
    postop: usize,
    preoff: f32,
    gain: f32,
    postoff: f32,
    from1: f32,
    from2: f32,
    to1: f32,
    to2: f32,
    integer: usize,
    do_clamp: bool,
    lo: f32,
    hi: f32,
) -> f32 {
    let mut v = math_unary(postop, v);
    v = (v + preoff) * gain + postoff;
    let span = (from2 - from1).max(1e-10);
    v = to1 + (v - from1) / span * (to2 - to1);
    v = match integer {
        1 => v.ceil(),
        2 => v.floor(),
        3 => v.round(),
        _ => v,
    };
    if do_clamp {
        v = v.clamp(lo.min(hi), hi.max(lo));
    }
    v
}

pub(crate) fn cook_math(c: &mut ChopCtx) -> ChopData {
    let preop = c.menu("preop");
    let chanop = c.menu("chanop");
    let chopop = c.menu("combine");
    let postop = c.menu("postop");
    let match_by = c.menu("match");
    let align = c.menu("align");
    let integer = c.menu("integer");
    let preoff = c.f("preoff");
    let gain = c.f("gain");
    let postoff = c.f("offset");
    let (from1, from2, to1, to2) = (
        c.f("fromrange1"),
        c.f("fromrange2"),
        c.f("torange1"),
        c.f("torange2"),
    );
    let (do_clamp, clo, chi) = (c.b("clamp"), c.f("clampmin"), c.f("clampmax"));
    let scope = c.s("scope");

    let a_raw = c.input(0).clone();
    let b_raw = c.input(1).clone();
    let has_a = !a_raw.channels.is_empty();
    let has_b = !b_raw.channels.is_empty();
    if !has_a && !has_b {
        return ChopData::empty();
    }

    let (a_in, b_in) = if has_a && has_b {
        math_align_inputs(a_raw.clone(), b_raw.clone(), align)
    } else if has_a {
        (a_raw.clone(), ChopData::empty())
    } else {
        (ChopData::empty(), b_raw.clone())
    };

    let a = math_prepare_input(a_in, preop, chanop);
    let b = math_prepare_input(b_in, preop, chanop);
    let len = a.num_samples().max(b.num_samples()).max(1);

    let mut channels = if chopop == 0 || !has_a || !has_b {
        let mut merged = a.channels;
        merged.extend(b.channels);
        merged
    } else if match_by == 1 {
        math_combine_chops_by_name(&a, &b, chopop, len)
    } else {
        math_combine_chops_by_index(&a, &b, chopop, len)
    };

    for ch in &mut channels {
        let pass = a_raw.channel(&ch.name).or_else(|| b_raw.channel(&ch.name));
        let scoped = otd_core::pattern_matches_channel(&scope, &ch.name);
        for (si, s) in ch.samples.iter_mut().enumerate() {
            if scoped {
                *s = math_finish_sample(
                    *s, postop, preoff, gain, postoff, from1, from2, to1, to2, integer, do_clamp,
                    clo, chi,
                );
            } else if let Some(src) = pass {
                *s = math_sample_at(src, si);
            }
        }
    }

    let rate = if a_raw.sample_rate > 0.0 {
        a_raw.sample_rate
    } else {
        b_raw.sample_rate.max(CONTROL_RATE)
    };
    ChopData::new(channels, rate, a_raw.time_sliced || b_raw.time_sliced)
}

// ------------------------------------------------------------------ lag

pub(crate) fn params_lag() -> IndexMap<String, Param> {
    params! {
        "lagup" => Param::float(0.1).with_label("Lag Up (s)").with_range(0.0, 2.0),
        "lagdown" => Param::float(0.1).with_label("Lag Down (s)").with_range(0.0, 2.0),
    }
}

/// One-pole coefficient for a time constant, per sample.
fn lag_alpha(tau: f32, sample_rate: f64) -> f32 {
    if tau <= 1e-5 {
        return 1.0;
    }
    let dt = 1.0 / sample_rate.max(1.0) as f32;
    1.0 - (-dt / tau).exp()
}

pub(crate) fn cook_lag(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let (up, down) = (c.f("lagup"), c.f("lagdown"));
    let (a_up, a_down) = (lag_alpha(up, rate), lag_alpha(down, rate));
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut y = c.state.f[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let alpha = if *x > y { a_up } else { a_down };
                y += (x - y) * alpha;
                y
            })
            .collect();
        c.state.f[ci] = y;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// --------------------------------------------------------------- filter

pub(crate) fn params_filter() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("lowpass", &["lowpass", "movingaverage"]).with_label("Type"),
        "width" => Param::float(0.2).with_label("Width (s)").with_range(0.0, 4.0),
    }
}

pub(crate) fn cook_filter(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let kind = c.menu("type");
    let width = c.f("width").max(0.0);
    c.state.fit(input.num_channels());

    if kind == 1 {
        // Moving average over the window, using the running value as the
        // history that predates this slice.
        let window = ((width as f64 * rate).round() as usize).max(1);
        let mut channels = Vec::with_capacity(input.num_channels());
        for (ci, ch) in input.channels.iter().enumerate() {
            let mut history = vec![c.state.f[ci]; window];
            let mut cursor = 0usize;
            let samples: Vec<f32> = ch
                .samples
                .iter()
                .map(|x| {
                    history[cursor] = *x;
                    cursor = (cursor + 1) % window;
                    history.iter().sum::<f32>() / window as f32
                })
                .collect();
            c.state.f[ci] = ch.last();
            channels.push(Channel::new(ch.name.clone(), samples));
        }
        return ChopData::new(channels, input.sample_rate, input.time_sliced);
    }

    let alpha = lag_alpha(width, rate);
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut y = c.state.f[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                y += (x - y) * alpha;
                y
            })
            .collect();
        c.state.f[ci] = y;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ---------------------------------------------------------------- logic

pub(crate) fn params_logic() -> IndexMap<String, Param> {
    params! {
        "convert" => Param::menu(
            "nonzero",
            &["off", "nonzero", "greater", "less", "equal"],
        ).with_label("Convert"),
        "threshold" => Param::float(0.5).with_label("Threshold"),
        "invert" => Param::bool(false).with_label("Invert"),
        "combine" => Param::menu("off", &["off", "and", "or", "xor"]).with_label("Combine Channels"),
    }
}

pub(crate) fn cook_logic(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let (convert, threshold, invert, combine_op) = (
        c.menu("convert"),
        c.f("threshold"),
        c.b("invert"),
        c.menu("combine"),
    );
    let mut out = map_samples(&input, |_, _, v| {
        let on = match convert {
            0 => return v,
            2 => v > threshold,
            3 => v < threshold,
            4 => (v - threshold).abs() < 1e-6,
            _ => v != 0.0,
        };
        let on = on != invert;
        if on { 1.0 } else { 0.0 }
    });

    if combine_op > 0 && out.num_channels() > 1 {
        let len = out.num_samples();
        let samples = (0..len)
            .map(|i| {
                let mut bits = out.channels.iter().map(|c| c.samples[i] != 0.0);
                let result = match combine_op {
                    1 => bits.all(|b| b),
                    2 => bits.any(|b| b),
                    _ => bits.fold(false, |a, b| a ^ b),
                };
                if result { 1.0 } else { 0.0 }
            })
            .collect();
        out = ChopData::new(
            vec![Channel::new("logic", samples)],
            out.sample_rate,
            out.time_sliced,
        );
    }
    out
}

// -------------------------------------------------------------- trigger

pub(crate) fn params_trigger() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Threshold"),
        "attack" => Param::float(0.05).with_label("Attack (s)").with_range(0.0, 2.0),
        "decay" => Param::float(0.2).with_label("Decay (s)").with_range(0.0, 4.0),
        "sustain" => Param::float(0.0).with_label("Sustain Level").with_range(0.0, 1.0),
        "release" => Param::float(0.3).with_label("Release (s)").with_range(0.0, 4.0),
    }
}

const STAGE_IDLE: i64 = 0;
const STAGE_ATTACK: i64 = 1;
const STAGE_SUSTAIN: i64 = 2;
const STAGE_RELEASE: i64 = 3;

pub(crate) fn cook_trigger(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let dt = 1.0 / rate as f32;
    let (threshold, attack, decay, sustain, release) = (
        c.f("threshold"),
        c.f("attack").max(1e-4),
        c.f("decay").max(1e-4),
        c.f("sustain").clamp(0.0, 1.0),
        c.f("release").max(1e-4),
    );
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut level = c.state.f[ci];
        let mut stage = c.state.i[ci];
        let mut prev = c.state.prev[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                // Rising edge through the threshold starts the envelope.
                if prev <= threshold && *x > threshold {
                    stage = STAGE_ATTACK;
                } else if prev > threshold && *x <= threshold && stage != STAGE_IDLE {
                    stage = STAGE_RELEASE;
                }
                prev = *x;

                match stage {
                    STAGE_ATTACK => {
                        level += dt / attack;
                        if level >= 1.0 {
                            level = 1.0;
                            stage = STAGE_SUSTAIN;
                        }
                    }
                    STAGE_SUSTAIN => {
                        if level > sustain {
                            level -= dt / decay * (1.0 - sustain).max(1e-6);
                            level = level.max(sustain);
                        }
                    }
                    STAGE_RELEASE => {
                        level -= dt / release;
                        if level <= 0.0 {
                            level = 0.0;
                            stage = STAGE_IDLE;
                        }
                    }
                    _ => {}
                }
                level
            })
            .collect();
        c.state.f[ci] = level;
        c.state.i[ci] = stage;
        c.state.prev[ci] = prev;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ---------------------------------------------------------------- timer

pub(crate) fn params_timer() -> IndexMap<String, Param> {
    with_rate(params! {
        "length" => Param::float(10.0).with_label("Length (s)").with_range(0.1, 600.0),
        "cycle" => Param::bool(true).with_label("Cycle"),
        "play" => Param::bool(true).with_label("Play"),
    })
}

pub(crate) fn cook_timer(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let dt = (c.time.dt / n as f64) as f32;
    let (length, cycle, play) = (c.f("length").max(1e-3), c.b("cycle"), c.b("play"));
    c.state.fit(1);

    let mut elapsed = c.state.f[0];
    let mut cycles = c.state.i[0];
    let mut done = 0.0f32;

    let (mut fraction, mut seconds, mut cyc, mut don) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    for _ in 0..n {
        if play {
            elapsed += dt;
            if elapsed >= length {
                if cycle {
                    elapsed -= length;
                    cycles += 1;
                } else {
                    elapsed = length;
                    done = 1.0;
                }
            }
        }
        fraction.push(elapsed / length);
        seconds.push(elapsed);
        cyc.push(cycles as f32);
        don.push(done);
    }
    c.state.f[0] = elapsed;
    c.state.i[0] = cycles;

    ChopData::new(
        vec![
            Channel::new("timer_fraction", fraction),
            Channel::new("timer_seconds", seconds),
            Channel::new("cycles", cyc),
            Channel::new("done", don),
        ],
        rate,
        true,
    )
}

// ---------------------------------------------------------------- speed

pub(crate) fn params_speed() -> IndexMap<String, Param> {
    params! {
        "gain" => Param::float(1.0).with_label("Gain").with_range(-4.0, 4.0),
        "initial" => Param::float(0.0).with_label("Initial Value"),
        "reset" => Param::bool(false).with_label("Reset"),
        "limit" => Param::menu("off", &["off", "clamp", "wrap"]).with_label("Limit"),
        "min" => Param::float(0.0).with_label("Min"),
        "max" => Param::float(1.0).with_label("Max"),
    }
}

pub(crate) fn cook_speed(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let dt = 1.0 / rate as f32;
    let (gain, initial, reset, limit, lo, hi) = (
        c.f("gain"),
        c.f("initial"),
        c.b("reset"),
        c.menu("limit"),
        c.f("min"),
        c.f("max"),
    );
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut pos = if reset { initial } else { c.state.f[ci] };
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|v| {
                pos += v * gain * dt;
                match limit {
                    1 => pos = pos.clamp(lo.min(hi), hi.max(lo)),
                    2 => {
                        let span = (hi - lo).abs().max(1e-6);
                        pos = lo + (pos - lo).rem_euclid(span);
                    }
                    _ => {}
                }
                pos
            })
            .collect();
        c.state.f[ci] = pos;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ---------------------------------------------------------------- count

pub(crate) fn params_count() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Threshold"),
        "edge" => Param::menu("rising", &["rising", "falling", "both"]).with_label("Edge"),
        "reset" => Param::bool(false).with_label("Reset"),
        "limit" => Param::menu("off", &["off", "clamp", "wrap"]).with_label("Limit"),
        "min" => Param::float(0.0).with_label("Min"),
        "max" => Param::float(8.0).with_label("Max"),
    }
}

pub(crate) fn cook_count(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let (threshold, edge, reset, limit, lo, hi) = (
        c.f("threshold"),
        c.menu("edge"),
        c.b("reset"),
        c.menu("limit"),
        c.f("min"),
        c.f("max"),
    );
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut count = if reset { 0.0 } else { c.state.f[ci] };
        let mut prev = c.state.prev[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let rising = prev <= threshold && *x > threshold;
                let falling = prev > threshold && *x <= threshold;
                let hit = match edge {
                    1 => falling,
                    2 => rising || falling,
                    _ => rising,
                };
                prev = *x;
                if hit {
                    count += 1.0;
                    match limit {
                        1 => count = count.clamp(lo.min(hi), hi.max(lo)),
                        2 => {
                            let span = (hi - lo).abs().max(1.0);
                            count = lo + (count - lo).rem_euclid(span);
                        }
                        _ => {}
                    }
                }
                count
            })
            .collect();
        c.state.f[ci] = count;
        c.state.prev[ci] = prev;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// --------------------------------------------------------------- select

pub(crate) fn params_select() -> IndexMap<String, Param> {
    params! {
        "channels" => Param::str("*").with_label("Channel Names"),
        "rename" => Param::str("").with_label("Rename To"),
    }
}

/// Space-separated names with `*` wildcards, TD style: `chan1 lfo*`.
pub fn name_matches(pattern: &str, name: &str) -> bool {
    otd_core::pattern_matches_channel(pattern, name)
}

pub(crate) fn cook_select(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let pattern = c.s("channels");
    let rename = c.s("rename");

    let channels: Vec<Channel> = input
        .channels
        .iter()
        .filter(|ch| pattern.trim().is_empty() || name_matches(&pattern, &ch.name))
        .enumerate()
        .map(|(i, ch)| Channel {
            name: otd_core::rename_to_at(&rename, i, &ch.name),
            samples: ch.samples.clone(),
        })
        .collect();
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ---------------------------------------------------------------- merge

pub(crate) fn cook_merge(c: &mut ChopCtx) -> ChopData {
    let a = c.input(0).clone();
    let b = c.input(1).clone();
    let len = a.num_samples().max(b.num_samples());
    let mut channels = a.channels.clone();
    channels.extend(b.channels.iter().cloned());

    // Duplicate names would make an Export ambiguous, so number them.
    let mut seen: Vec<String> = Vec::new();
    for ch in &mut channels {
        if seen.contains(&ch.name) {
            let mut i = 1;
            loop {
                let candidate = format!("{}{}", ch.name, i);
                if !seen.contains(&candidate) {
                    ch.name = candidate;
                    break;
                }
                i += 1;
            }
        }
        seen.push(ch.name.clone());
    }
    let mut out = ChopData::new(
        channels,
        a.sample_rate.max(b.sample_rate).max(CONTROL_RATE),
        a.time_sliced || b.time_sliced,
    );
    out.pad_to(len);
    out
}

// --------------------------------------------------------------- switch

pub(crate) fn params_switch() -> IndexMap<String, Param> {
    params! {
        "index" => Param::float(0.0).with_label("Index").with_range(0.0, 1.0),
    }
}

pub(crate) fn cook_switch(c: &mut ChopCtx) -> ChopData {
    let index = c.f("index");
    if index >= 0.5 {
        c.input(1).clone()
    } else {
        c.input(0).clone()
    }
}

// ----------------------------------------------------------------- null

pub(crate) fn cook_null(c: &mut ChopCtx) -> ChopData {
    c.input(0).clone()
}

// ---------------------------------------------------------------- delay

pub(crate) fn params_delay() -> IndexMap<String, Param> {
    params! {
        "delay" => Param::float(0.25).with_label("Delay (s)").with_range(0.0, 10.0),
        "mix" => Param::float(1.0).with_label("Mix").with_range(0.0, 1.0),
        "feedback" => Param::float(0.0).with_label("Feedback").with_range(0.0, 0.95),
    }
}

/// Hold each channel's history and read it back later.
///
/// The buffer is per channel and lives in the node's state, which is what
/// makes this a CHOP rather than a parameter trick: the delay is in *samples*
/// of the input's own rate, so the same node delays a 60 Hz control channel
/// and a 48 kHz audio channel by the same wall-clock time.
pub(crate) fn cook_delay(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let want = ((c.f("delay") as f64) * rate).round().max(0.0) as usize;
    let mix = c.f("mix").clamp(0.0, 1.0);
    let fb = c.f("feedback").clamp(0.0, 0.95);

    if want == 0 {
        return input;
    }
    // One ring per channel, laid end to end in the state's float scratch so
    // the existing per-node state carries it with no new machinery. The ring
    // is exactly `want` long: read the oldest slot, overwrite it, then
    // advance, which makes sample *n* come out as sample *n - want* rather
    // than one late.
    let stride = want;
    let needed = input.num_channels() * stride;
    if c.state.f.len() != needed {
        c.state.f.clear();
        c.state.f.resize(needed, 0.0);
        c.state.i.clear();
        c.state.i.resize(input.num_channels().max(1), 0);
    }
    c.state.i.resize(input.num_channels().max(1), 0);

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let base = ci * stride;
        let mut head = c.state.i[ci] as usize;
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let delayed = c.state.f[base + head];
                c.state.f[base + head] = x + delayed * fb;
                head = (head + 1) % stride;
                x * (1.0 - mix) + delayed * mix
            })
            .collect();
        c.state.i[ci] = head as i64;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ---------------------------------------------------------------- trail

pub(crate) fn params_trail() -> IndexMap<String, Param> {
    params! {
        "window" => Param::float(2.0).with_label("Window (s)").with_range(0.05, 60.0),
    }
}

/// The last `window` seconds of every channel, oldest first, as one buffer.
///
/// The ring lives in the float scratch like Delay's, but the output is the
/// whole history rather than one late sample — which is what a waveform
/// line or a per-sample instance layout wants to draw.
pub(crate) fn cook_trail(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    // ponytail: 65536 samples per channel caps a 48 kHz trail at ~1.4 s.
    let len = ((c.f("window") as f64 * rate).round() as usize).clamp(2, 1 << 16);
    let needed = input.num_channels() * len;
    if c.state.f.len() != needed {
        c.state.f = vec![0.0; needed];
        c.state.i = vec![0; input.num_channels().max(1)];
    }
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let ring = &mut c.state.f[ci * len..(ci + 1) * len];
        let mut head = c.state.i[ci] as usize;
        for x in &ch.samples {
            ring[head] = *x;
            head = (head + 1) % len;
        }
        c.state.i[ci] = head as i64;
        let samples = ring[head..].iter().chain(&ring[..head]).copied().collect();
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, false)
}

// --------------------------------------------------------------- spring

pub(crate) fn params_spring() -> IndexMap<String, Param> {
    params! {
        "spring" => Param::float(80.0).with_label("Spring Constant").with_range(0.0, 1000.0),
        "damping" => Param::float(6.0).with_label("Damping").with_range(0.0, 100.0),
    }
}

/// Follow the input like a mass on a spring: overshoot, wobble, settle.
///
/// Unlike Lag this keeps moving after the input stops, so it has to cook
/// every frame. Position is `state.f`, velocity is `state.prev`, and
/// `state.i` marks a channel as started so the first cook begins at rest on
/// the input instead of flying in from zero.
pub(crate) fn cook_spring(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let dt = 1.0 / input.sample_rate.max(1.0) as f32;
    let (k, d) = (c.f("spring").max(0.0), c.f("damping").max(0.0));
    c.state.fit(input.num_channels());
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        if c.state.i[ci] == 0 {
            c.state.f[ci] = ch.samples.first().copied().unwrap_or(0.0);
            c.state.prev[ci] = 0.0;
            c.state.i[ci] = 1;
        }
        let (mut y, mut v) = (c.state.f[ci], c.state.prev[ci]);
        let samples = ch
            .samples
            .iter()
            .map(|x| {
                // Semi-implicit Euler: stable for the stiffness range offered.
                v += (k * (x - y) - d * v) * dt;
                y += v * dt;
                y
            })
            .collect();
        (c.state.f[ci], c.state.prev[ci]) = (y, v);
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ------------------------------------------------------------------ fan

pub(crate) fn params_fan() -> IndexMap<String, Param> {
    params! {
        "operation" => Param::menu("fanout", &["fanout", "fanin"]).with_label("Operation"),
        "channels" => Param::int(4).with_label("Channels (fan out)").with_range(1.0, 64.0),
    }
}

/// Fan out: one index channel becomes `chan0..chanN-1`, 1 where the rounded
/// index matches. Fan in: many channels become `chan1`, the index of the
/// first one above zero, or -1 when none is.
pub(crate) fn cook_fan(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let n = input.channels.first().map_or(1, |ch| ch.samples.len()).max(1);
    let channels = if c.menu("operation") == 0 {
        let count = c.i("channels").clamp(1, 64) as usize;
        let index = input.channels.first();
        (0..count)
            .map(|k| {
                let samples = (0..n)
                    .map(|s| {
                        let v = index.and_then(|ch| ch.samples.get(s)).copied().unwrap_or(-1.0);
                        if v.round() == k as f32 { 1.0 } else { 0.0 }
                    })
                    .collect();
                Channel::new(format!("chan{k}"), samples)
            })
            .collect()
    } else {
        let samples = (0..n)
            .map(|s| {
                input
                    .channels
                    .iter()
                    .position(|ch| ch.samples.get(s).is_some_and(|v| *v > 0.0))
                    .map_or(-1.0, |i| i as f32)
            })
            .collect();
        vec![Channel::new("chan1", samples)]
    };
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ----------------------------------------------------------- expression

pub(crate) fn params_expression() -> IndexMap<String, Param> {
    params! {
        "expr" => Param::str("v").with_label("Expression"),
    }
}

/// An arbitrary expression applied to every sample.
///
/// `v` is the sample, `i` its index, `c` the channel index and `n` the number
/// of samples; `time`, `frame` and the rest of the evaluator's vocabulary are
/// in scope too, so `v * sin(time)` is a whole operator.
///
/// The expression is parsed **once per cook**, not once per sample. That is
/// the difference between an operator you can put on an audio-rate channel and
/// one you cannot: parsing 512 times a frame would cost more than the
/// arithmetic by two orders of magnitude.
pub(crate) fn cook_expression(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let source = c.s("expr");
    let Ok(expr) = otd_core::Expr::parse(&source) else {
        // A half-typed expression holds the input rather than emptying the
        // chain — the same rule the GLSL TOP follows for a shader mid-edit.
        return input;
    };

    let n = input.num_samples() as f64;
    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let locals = [
                    ("v", *v as f64),
                    ("i", i as f64),
                    ("c", ci as f64),
                    ("n", n),
                ];
                let eval = otd_core::EvalContext {
                    locals: &locals,
                    ..*c.eval
                };
                expr.eval(&eval).map(|r| r as f32).unwrap_or(*v)
            })
            .collect();
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ---------------------------------------------------------------- limit

pub(crate) fn params_limit() -> IndexMap<String, Param> {
    params! {
        "type" => Param::menu("clamp", &["clamp", "loop", "zigzag", "quantise"])
            .with_label("Type"),
        "min" => Param::float(0.0).with_label("Minimum").with_range(-10.0, 10.0),
        "max" => Param::float(1.0).with_label("Maximum").with_range(-10.0, 10.0),
        "step" => Param::float(0.1).with_label("Quantise Step").with_range(0.0, 10.0),
    }
}

pub(crate) fn cook_limit(c: &mut ChopCtx) -> ChopData {
    let (lo, hi) = (c.f("min"), c.f("max"));
    let (lo, hi) = if lo <= hi { (lo, hi) } else { (hi, lo) };
    let span = (hi - lo).max(1e-6);
    let step = c.f("step");
    let mode = c.menu("type");
    map_samples(c.input(0), |_, _, v| match mode {
        0 => v.clamp(lo, hi),
        1 => lo + (v - lo).rem_euclid(span),
        2 => {
            // Fold instead of wrap: a value walking past the top comes back
            // down rather than jumping, which is the whole point of zigzag.
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
    })
}

// ---------------------------------------------------------------- slope

pub(crate) fn params_slope() -> IndexMap<String, Param> {
    params! {
        "units" => Param::menu("persecond", &["persecond", "persample"])
            .with_label("Units"),
    }
}

/// The derivative — the inverse of the Speed CHOP, which integrates.
pub(crate) fn cook_slope(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let scale = if c.menu("units") == 0 {
        input.sample_rate.max(1.0) as f32
    } else {
        1.0
    };
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        // Carrying the last sample across the frame boundary is what keeps
        // the slope continuous; without it every frame starts with a spike.
        let mut prev = c.state.prev[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                let d = (x - prev) * scale;
                prev = *x;
                d
            })
            .collect();
        c.state.prev[ci] = prev;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// ----------------------------------------------------------------- hold

pub(crate) fn params_hold() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Trigger Threshold").with_range(-10.0, 10.0),
    }
}

/// Sample and hold: input 0's value is frozen until input 1 rises past the
/// threshold. Two inputs rather than a parameter because the thing that says
/// "now" is nearly always another channel — a beat, a MIDI note, a button.
pub(crate) fn cook_hold(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let trigger = c.input(1).clone();
    let threshold = c.f("threshold");
    c.state.fit(input.num_channels() + 1);
    let n = input.num_samples();

    // Sample index of every rising edge in this slice.
    let mut fires = Vec::new();
    if let Some(t) = trigger.channels.first() {
        let mut prev = c.state.prev[input.num_channels()];
        for (i, v) in t.samples.iter().enumerate() {
            if prev < threshold && *v >= threshold {
                fires.push(i);
            }
            prev = *v;
        }
        c.state.prev[input.num_channels()] = prev;
    }

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut held = c.state.f[ci];
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            if fires.contains(&i) {
                held = ch.samples.get(i).copied().unwrap_or(held);
            }
            out.push(held);
        }
        c.state.f[ci] = held;
        channels.push(Channel::new(ch.name.clone(), out));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// --------------------------------------------------------------- shuffle

pub(crate) fn params_shuffle() -> IndexMap<String, Param> {
    params! {
        "method" => Param::menu(
            "transpose",
            &["transpose", "reverse", "swapfirstlast", "sequence"],
        ).with_label("Method"),
    }
}

/// Rearrange the channel/sample grid.
///
/// Transpose is the one that earns its keep: N channels of one sample become
/// one channel of N samples, which is how a pile of separate values becomes
/// something a CHOP to TOP or an instancing Geometry COMP can consume.
pub(crate) fn cook_shuffle(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let n = input.num_samples();
    let channels = match c.menu("method") {
        0 => {
            let rows: Vec<Channel> = (0..n)
                .map(|s| {
                    Channel::new(
                        format!("chan{}", s + 1),
                        input
                            .channels
                            .iter()
                            .map(|ch| ch.samples.get(s).copied().unwrap_or(0.0))
                            .collect(),
                    )
                })
                .collect();
            rows
        }
        1 => input
            .channels
            .iter()
            .map(|ch| {
                let mut s = ch.samples.clone();
                s.reverse();
                Channel::new(ch.name.clone(), s)
            })
            .collect(),
        2 => {
            let mut out = input.channels.clone();
            if out.len() >= 2 {
                let last = out.len() - 1;
                out.swap(0, last);
            }
            out
        }
        _ => {
            // Every channel end to end in one.
            let mut all = Vec::new();
            for ch in &input.channels {
                all.extend_from_slice(&ch.samples);
            }
            vec![Channel::new("chan1", all)]
        }
    };
    // Transposing a time-sliced stream produces a buffer, not a stream.
    ChopData::new(channels, input.sample_rate, false)
}

// ---------------------------------------------------------------- rename

pub(crate) fn params_rename() -> IndexMap<String, Param> {
    params! {
        "from" => Param::str("*").with_label("Rename Channels"),
        "to" => Param::str("chan[1-]").with_label("To"),
    }
}

/// Rename channels, TD's numbered-pattern style: `chan[1-]` names them
/// `chan1`, `chan2`, … A plain name with no bracket renames every match to
/// the same thing, which is what you want feeding a Merge.
pub(crate) fn cook_rename(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let from = c.s("from");
    let to = c.s("to");
    let patterns: Vec<&str> = from.split_whitespace().collect();

    let numbered = otd_core::parse_numbered_pattern(&to);
    let mut rename_index = 0usize;
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            if !patterns.iter().any(|p| otd_core::pattern_matches_channel(p, &ch.name)) {
                return ch.clone();
            }
            let name = if let Some(pat) = numbered {
                let n = otd_core::channel_name_at(&pat, rename_index);
                rename_index += 1;
                n
            } else {
                to.clone()
            };
            Channel::new(name, ch.samples.clone())
        })
        .collect();
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}

// --------------------------------------------------------------- analyze

pub(crate) fn params_analyze() -> IndexMap<String, Param> {
    params! {
        "function" => Param::menu(
            "average",
            &["average", "maximum", "minimum", "rms", "sum", "length", "median"],
        ).with_label("Function"),
    }
}

/// Reduce each channel to one number.
///
/// A whole waveform is rarely what a parameter wants; one number that
/// describes it usually is. Keeping the channel names means a downstream
/// export still reads `ch('/analyze1', 'chan1')` and gets the summary.
pub(crate) fn cook_analyze(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let func = c.menu("function");
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            let s = &ch.samples;
            let n = s.len().max(1) as f32;
            let v = match func {
                0 => s.iter().sum::<f32>() / n,
                1 => ch.max(),
                2 => ch.min(),
                3 => (s.iter().map(|v| v * v).sum::<f32>() / n).sqrt(),
                4 => s.iter().sum::<f32>(),
                5 => s.len() as f32,
                _ => {
                    let mut sorted = s.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    sorted.get(sorted.len() / 2).copied().unwrap_or(0.0)
                }
            };
            Channel::new(ch.name.clone(), vec![v])
        })
        .collect();
    ChopData::new(channels, input.sample_rate, false)
}

// ----------------------------------------------------------------- cross

pub(crate) fn params_cross() -> IndexMap<String, Param> {
    params! {
        "cross" => Param::float(0.5).with_label("Cross").with_range(0.0, 1.0),
        "curve" => Param::menu("linear", &["linear", "equalpower", "smooth"])
            .with_label("Curve"),
    }
}

/// Blend two inputs channel by channel.
///
/// Equal power is the default worth knowing about: a linear crossfade between
/// two uncorrelated signals dips ~3 dB in the middle, which is audible on
/// audio and visible on anything driving brightness.
pub(crate) fn cook_cross(c: &mut ChopCtx) -> ChopData {
    let a = c.input(0).clone();
    let b = c.input(1).clone();
    let t = c.f("cross").clamp(0.0, 1.0);
    let quarter = std::f32::consts::FRAC_PI_2;
    let (wa, wb) = match c.menu("curve") {
        1 => (((1.0 - t) * quarter).sin(), (t * quarter).sin()),
        2 => {
            let s = t * t * (3.0 - 2.0 * t);
            (1.0 - s, s)
        }
        _ => (1.0 - t, t),
    };

    let len = a.num_samples().max(b.num_samples());
    let count = a.num_channels().max(b.num_channels());
    let channels = (0..count)
        .map(|ci| {
            let ca = a.nth(ci);
            let cb = b.nth(ci);
            let name = ca
                .or(cb)
                .map(|c| c.name.clone())
                .unwrap_or_else(|| format!("chan{}", ci + 1));
            let samples = (0..len)
                .map(|i| {
                    let va = ca.map(|c| sample_held(&c.samples, i)).unwrap_or(0.0);
                    let vb = cb.map(|c| sample_held(&c.samples, i)).unwrap_or(0.0);
                    va * wa + vb * wb
                })
                .collect();
            Channel::new(name, samples)
        })
        .collect();
    ChopData::new(channels, a.sample_rate.max(b.sample_rate), a.time_sliced)
}

/// Read a sample, holding the last one past the end.
///
/// This is what lets a single-sample control channel broadcast against a
/// waveform — the same rule instancing uses, so it does not surprise.
fn sample_held(s: &[f32], i: usize) -> f32 {
    s.get(i).or_else(|| s.last()).copied().unwrap_or(0.0)
}

// -------------------------------------------------------------- resample

pub(crate) fn params_resample() -> IndexMap<String, Param> {
    params! {
        "length" => Param::int(64).with_label("Length (samples)").with_range(1.0, 8192.0),
        "interp" => Param::menu("linear", &["linear", "nearest"]).with_label("Interpolate"),
    }
}

/// Change how many samples a buffer has, keeping its shape.
pub(crate) fn cook_resample(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let n = c.i("length").clamp(1, 8192) as usize;
    let nearest = c.menu("interp") == 1;
    let channels = input
        .channels
        .iter()
        .map(|ch| {
            let src = &ch.samples;
            let samples = (0..n)
                .map(|i| {
                    if src.is_empty() {
                        return 0.0;
                    }
                    if src.len() == 1 {
                        return src[0];
                    }
                    let pos = i as f32 * (src.len() - 1) as f32 / (n.max(2) - 1) as f32;
                    let lo = pos.floor() as usize;
                    if nearest {
                        return src[pos.round() as usize % src.len()];
                    }
                    let hi = (lo + 1).min(src.len() - 1);
                    let f = pos - lo as f32;
                    src[lo] * (1.0 - f) + src[hi] * f
                })
                .collect();
            Channel::new(ch.name.clone(), samples)
        })
        .collect();
    // A resampled buffer is no longer one frame's worth of a stream.
    ChopData::new(channels, input.sample_rate, false)
}

// ----------------------------------------------------------------- clock

pub(crate) fn params_clock() -> IndexMap<String, Param> {
    crate::clock::params_clock()
}

pub(crate) fn cook_clock(c: &mut ChopCtx) -> ChopData {
    crate::clock::cook_clock(c)
}

// ------------------------------------------------------------------ beat

pub(crate) fn params_beat() -> IndexMap<String, Param> {
    with_rate(params! {
        "tempo" => Param::float(120.0).with_label("Tempo (BPM)").with_range(20.0, 300.0),
        "beatsperbar" => Param::int(4).with_label("Beats Per Bar").with_range(1.0, 32.0),
    })
}

/// A tempo clock: `ramp` 0..1 within a beat, `pulse` at each beat, `count`,
/// and the same for a bar.
///
/// Derived from the timeline's absolute time rather than from a counter, so a
/// dropped frame does not put the beat permanently behind — the same reason
/// the Animation CHOP and Movie File In are functions of time.
pub(crate) fn cook_beat(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let bps = (c.f("tempo") as f64 / 60.0).max(1e-6);
    let per_bar = c.i("beatsperbar").max(1) as f64;

    let mut ramp = Vec::with_capacity(n);
    let mut pulse = Vec::with_capacity(n);
    let mut count = Vec::with_capacity(n);
    let mut bar_ramp = Vec::with_capacity(n);
    let mut prev = c.state.f.first().copied().unwrap_or(0.0);
    c.state.fit(1);

    for t in slice_times(c.time, n) {
        let beats = t * bps;
        let frac = beats.rem_euclid(1.0) as f32;
        ramp.push(frac);
        // A pulse is one sample wide at the wrap, which is what a Trigger or
        // a Count downstream is looking for.
        pulse.push(if frac < prev { 1.0 } else { 0.0 });
        prev = frac;
        count.push(beats.floor() as f32);
        bar_ramp.push((beats / per_bar).rem_euclid(1.0) as f32);
    }
    c.state.f[0] = prev;

    ChopData::new(
        vec![
            Channel::new("ramp", ramp),
            Channel::new("pulse", pulse),
            Channel::new("count", count),
            Channel::new("bar", bar_ramp),
        ],
        rate,
        true,
    )
}

// ----------------------------------------------------------------- panel

pub(crate) fn params_panel() -> IndexMap<String, Param> {
    params! {
        "ops" => Param::str("").with_label("Panel COMPs (paths)"),
    }
}

/// The values of some panel widgets, as channels.
///
/// A widget's state is an ordinary parameter, so Bind mode already reaches it
/// and this operator is a convenience rather than a mechanism. It earns its
/// place when there are eight faders: one node with eight channels beats eight
/// parameters each bound separately, and the result filters, lags and exports
/// like anything else.
///
/// Paths rather than a search: enumerating the graph is not something the
/// narrow view a CHOP gets should be widened to allow, and naming what you
/// want is a line of text.
pub(crate) fn cook_panel(c: &mut ChopCtx) -> ChopData {
    let paths = c.s("ops");
    let channels: Vec<Channel> = paths
        .split_whitespace()
        .map(|path| {
            let value = c
                .eval
                .channels
                .and_then(|net| net.param_value(path, "value"))
                .map(|v| v.as_f32())
                .unwrap_or(0.0);
            // Named after the node, not the whole path, so `/panel/fader1`
            // reads as `fader1` in an expression.
            let name = path.rsplit('/').next().unwrap_or(path);
            Channel::constant(sanitise(name), value, 1)
        })
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}

// -------------------------------------------------------- DAT to CHOP

pub(crate) fn params_particles() -> IndexMap<String, Param> {
    params! {
        "count" => Param::int(256).with_label("Particle Count").with_range(1.0, 4096.0),
        "emitx" => Param::float(0.5).with_label("Emitter X (0..1)").with_range(0.0, 1.0),
        "emity" => Param::float(0.5).with_label("Emitter Y (0..1, down)").with_range(0.0, 1.0),
        "speed" => Param::float(0.2).with_label("Speed (frame widths/s)").with_range(0.0, 2.0),
        "gravity" => Param::float(0.2).with_label("Downward Acceleration").with_range(-2.0, 2.0),
        "lifetime" => Param::float(2.0).with_label("Lifetime (s)").with_range(0.1, 20.0),
        "active" => Param::bool(true).with_label("Emit"),
        "reset" => Param::bool(false).with_label("Reset"),
    }
}

pub(crate) fn cook_particles(c: &mut ChopCtx) -> ChopData {
    let n = c.i("count").clamp(1, 4096) as usize;
    let lifetime = c.f("lifetime").clamp(0.1, 20.0);
    let (emitx, emity, speed, gravity) = (c.f("emitx"), c.f("emity"), c.f("speed"), c.f("gravity"));
    let active = c.b("active");
    let rewind = c.state.prev.first().is_some_and(|t| c.time.abs_time < *t as f64);
    if c.state.f.len() != n * 5 || c.b("reset") || rewind {
        c.state.f = vec![0.0; n * 5];
        for p in c.state.f.chunks_mut(5) { p[4] = lifetime; }
        c.state.i = vec![0; n];
    }
    c.state.prev = vec![c.time.abs_time as f32];
    let dt = c.time.dt.clamp(0.0, 0.1) as f32;
    let mut output: [Vec<f32>; 5] = std::array::from_fn(|_| Vec::with_capacity(n));
    for i in 0..n {
        let p = &mut c.state.f[i * 5..i * 5 + 5];
        if p[4] >= lifetime && active {
            c.state.i[i] += 1;
            let seed = (i as u32).wrapping_mul(747796405)
                .wrapping_add(c.state.i[i] as u32 * 2891336453u32.wrapping_div(4));
            let angle = (seed % 65536) as f32 / 65536.0 * std::f32::consts::TAU;
            p[0] = emitx; p[1] = emity;
            p[2] = angle.cos() * speed; p[3] = angle.sin() * speed;
            p[4] = if c.state.i[i] == 1 { lifetime * i as f32 / n as f32 } else { 0.0 };
        }
        if p[4] < lifetime {
            p[0] += p[2] * dt;
            p[1] += p[3] * dt + 0.5 * gravity * dt * dt;
            p[3] += gravity * dt;
            p[4] += dt;
        }
        let life = (1.0 - p[4] / lifetime).clamp(0.0, 1.0);
        output[0].push((p[0] - 0.5) * 6.0);
        output[1].push((0.5 - p[1]) * 6.0);
        output[2].push(0.0);
        output[3].push(life);
        output[4].push(life);
    }
    ChopData::new(["tx", "ty", "tz", "size", "life"].into_iter()
        .zip(output).map(|(name, samples)| Channel::new(name, samples)).collect(), CONTROL_RATE, false)
}

pub(crate) fn params_blobtrack() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Luminance Threshold").with_range(0.0, 1.0),
        "minarea" => Param::float(0.002).with_label("Minimum Frame Fraction").with_range(0.0, 1.0),
    }
}

pub(crate) fn cook_blobtrack(c: &mut ChopCtx) -> ChopData {
    let blob = match c.foreign(0) {
        Some(Crossing::Pixels { width, height, rgba }) => crate::tracking::largest(
            *width as usize, *height as usize, rgba, c.f("threshold"), c.f("minarea")),
        _ => crate::tracking::Blob::default(),
    };
    let present = blob.count > 0;
    let previous = c.state.prev.as_slice();
    let velocity = if present && previous.len() == 3 && previous[2] > 0.0 {
        let dt = c.time.dt.max(0.000001) as f32;
        [(blob.x - previous[0]) / dt, (blob.y - previous[1]) / dt]
    } else { [0.0, 0.0] };
    c.state.prev = vec![blob.x, blob.y, if present {1.0} else {0.0}];
    ChopData::new([
        ("present", if present {1.0} else {0.0}), ("count", blob.count as f32),
        ("x", blob.x), ("y", blob.y), ("area", blob.area),
        ("width", blob.width), ("height", blob.height),
        ("vx", velocity[0]), ("vy", velocity[1]),
    ].into_iter().map(|(name, value)| Channel::constant(name, value, 1)).collect(),
        CONTROL_RATE, false)
}

pub(crate) fn params_dat_to() -> IndexMap<String, Param> {
    params! {
        "layout" => Param::menu("columns", &["columns", "rows"])
            .with_label("Channels From"),
        "names" => Param::bool(true)
            .with_label("First Row/Column Is Names"),
    }
}

/// Numbers out of a table. A cue list, a CSV of positions, a JSON payload
/// already flattened by a JSON DAT — all of them become channels here, which
/// is the point of the DAT family being a family at all.
pub(crate) fn cook_dat_to_chop(c: &mut ChopCtx) -> ChopData {
    let Some(rows) = c.foreign(0).and_then(|f| f.as_table()) else {
        return ChopData::empty();
    };
    let by_columns = c.menu("layout") == 0;
    let named = c.b("names");

    // Work in "lines of cells", where a line is a column or a row depending
    // on the layout, so the two cases share everything below.
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let cell = |r: usize, col: usize| -> &str {
        rows.get(r)
            .and_then(|row| row.get(col))
            .map(|s| s.as_str())
            .unwrap_or("")
    };
    let lines: Vec<Vec<String>> = if by_columns {
        (0..width)
            .map(|col| (0..rows.len()).map(|r| cell(r, col).to_string()).collect())
            .collect()
    } else {
        rows.iter().map(|r| r.to_vec()).collect()
    };

    let channels: Vec<Channel> = lines
        .into_iter()
        .enumerate()
        .map(|(i, mut line)| {
            let name = if named && !line.is_empty() {
                let head = line.remove(0);
                if head.trim().is_empty() {
                    format!("chan{}", i + 1)
                } else {
                    sanitise(&head)
                }
            } else {
                format!("chan{}", i + 1)
            };
            // A cell that is not a number is 0 rather than an error: a table
            // usually has a label column, and refusing to cook because of it
            // would make the common case the broken one.
            let samples = line
                .iter()
                .map(|s| s.trim().parse().unwrap_or(0.0))
                .collect();
            Channel::new(name, samples)
        })
        .filter(|c| !c.samples.is_empty())
        .collect();

    // Not time sliced: this is a standalone buffer, like a Pattern CHOP.
    ChopData::new(channels, CONTROL_RATE, false)
}

/// A channel name has to survive being typed into an expression.
fn sanitise(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    if cleaned.starts_with(|ch: char| ch.is_ascii_digit()) {
        format!("_{cleaned}")
    } else if cleaned.is_empty() {
        "chan".into()
    } else {
        cleaned
    }
}

// -------------------------------------------------------- SOP to CHOP

pub(crate) fn params_sop_to() -> IndexMap<String, Param> {
    params! {
        "attrs" => Param::str("P")
            .with_label("Attributes (P N uv Cd)"),
    }
}

/// Geometry as channels: one sample per point, one channel per component.
///
/// This is what closes the instancing loop. A Copy SOP or a Noise SOP can
/// decide where 256 things go, and the positions come back out here as the
/// `tx`/`ty`/`tz` an instancing Geometry COMP reads — geometry driving
/// geometry, without either end knowing about the other.
pub(crate) fn cook_sop_to_chop(c: &mut ChopCtx) -> ChopData {
    let Some(points) = c.foreign(0) else {
        return ChopData::empty();
    };
    let stride = points.point_stride();
    let Crossing::Points { data, .. } = points else {
        return ChopData::empty();
    };
    if stride == 0 {
        return ChopData::empty();
    }
    let count = data.len() / stride;

    let wanted = c.s("attrs");
    let wanted: Vec<&str> = wanted.split_whitespace().collect();
    let suffix = ["x", "y", "z", "w"];

    let mut channels = Vec::new();
    for name in wanted {
        let Some((offset, width)) = points.attr(name) else {
            continue;
        };
        for comp in 0..width {
            let samples = (0..count)
                .map(|p| data[p * stride + offset + comp])
                .collect();
            let label = if width == 1 {
                name.to_string()
            } else {
                format!("{name}{}", suffix.get(comp).copied().unwrap_or("?"))
            };
            channels.push(Channel::new(label, samples));
        }
    }
    ChopData::new(channels, CONTROL_RATE, false)
}

// -------------------------------------------------------- TOP to CHOP

pub(crate) fn params_top_to() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "layout" => Param::menu("rows", &["rows", "columns", "average", "image"])
            .with_label("Read"),
        "index" => Param::int(0)
            .with_label("Row / Column")
            .with_range(0.0, 4096.0),
    }
}

/// Pixels as channels: `r`, `g`, `b`, `a`, one sample per pixel read.
///
/// Two things about this operator are worth knowing before you patch it, and
/// both are true of TouchDesigner's as well:
///
///  * It reads **last frame**. A readback has to be submitted to the GPU, and
///    this frame's work is still being recorded into an encoder that has not
///    been submitted yet. Waiting for it would mean stalling the frame on the
///    render it is in the middle of building.
///  * It costs a full pipeline sync, and the cost is the *source's*
///    resolution. Put a Resolution TOP in front of it. Reading 1920×1080 back
///    every frame to get four numbers is the classic way to turn a 60 fps
///    patch into a 12 fps one.
pub(crate) fn cook_top_to_chop(c: &mut ChopCtx) -> ChopData {
    if !c.b("active") {
        return ChopData::empty();
    }
    let Some(Crossing::Pixels {
        width,
        height,
        rgba,
    }) = c.foreign(0)
    else {
        return ChopData::empty();
    };
    let (w, h) = (*width as usize, *height as usize);
    if w == 0 || h == 0 {
        return ChopData::empty();
    }
    let at = |x: usize, y: usize, comp: usize| rgba[(y * w + x) * 4 + comp];

    let mut samples = vec![Vec::new(); 4];
    match c.menu("layout") {
        // One row of pixels, left to right.
        0 => {
            let y = (c.i("index").max(0) as usize).min(h - 1);
            for (comp, out) in samples.iter_mut().enumerate() {
                *out = (0..w).map(|x| at(x, y, comp)).collect();
            }
        }
        // One column, top to bottom.
        1 => {
            let x = (c.i("index").max(0) as usize).min(w - 1);
            for (comp, out) in samples.iter_mut().enumerate() {
                *out = (0..h).map(|y| at(x, y, comp)).collect();
            }
        }
        // Every pixel, top row first, plus where it is: `u` across and `v`
        // up, centred on zero with `u` spanning the aspect ratio. Named so a
        // Geometry COMP can instance straight off it — tx=u, ty=v, tz=g is a
        // luma-displaced point cloud of the picture with no other operator.
        3 => {
            // ponytail: 16384 pixels (128x128) caps the per-frame CPU copy;
            // a GPU instancing path is the upgrade.
            if w * h > 1 << 14 {
                return ChopData::empty();
            }
            for (comp, out) in samples.iter_mut().enumerate() {
                *out = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| at(x, y, comp)).collect();
            }
            let aspect = w as f32 / h as f32;
            let (u, v): (Vec<f32>, Vec<f32>) = (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| (
                    ((x as f32 + 0.5) / w as f32 - 0.5) * aspect,
                    0.5 - (y as f32 + 0.5) / h as f32,
                ))
                .unzip();
            samples.push(u);
            samples.push(v);
        }
        // The whole image as one number per component.
        _ => {
            let n = (w * h) as f32;
            for (comp, out) in samples.iter_mut().enumerate() {
                let sum: f32 = (0..h)
                    .flat_map(|y| (0..w).map(move |x| (x, y)))
                    .map(|(x, y)| at(x, y, comp))
                    .sum();
                *out = vec![sum / n];
            }
        }
    }

    let names = ["r", "g", "b", "a", "u", "v"];
    let channels = samples
        .into_iter()
        .enumerate()
        .map(|(i, s)| Channel::new(names[i], s))
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}
