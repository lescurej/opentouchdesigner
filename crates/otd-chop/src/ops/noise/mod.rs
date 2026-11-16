use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData, slice_len};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
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

#[allow(clippy::too_many_arguments)]
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

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
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



pub const TYPE_NAME: &str = "noiseCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Noise",
                &[],
                "Sparse convolution noise over time; channel count comes from Channel Names.",
                params,
                cook,
            )
}
