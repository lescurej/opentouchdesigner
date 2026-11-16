use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};


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

pub(crate) fn params() -> IndexMap<String, Param> {
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

#[allow(clippy::too_many_arguments)]
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

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
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



pub const TYPE_NAME: &str = "mathCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Math",
                &["a", "b"],
                "Combine and scale channels.",
                params,
                cook,
            )
}
