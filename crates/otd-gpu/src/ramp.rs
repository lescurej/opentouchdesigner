//! Ramp TOP — keyframe table parsing and LUT baking (CPU).
//!
//! TouchDesigner stores arbitrary stops in a DAT; the GPU sees a 1×N RGBA8
//! lookup texture bound as input 2.

use otd_core::{EvalContext, Node};

pub const LUT_WIDTH: u32 = 512;

#[derive(Clone, Debug, PartialEq)]
pub struct RampKeyframe {
    pub pos: f32,
    pub rgba: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RampInterp {
    Step = 0,
    Linear = 1,
    EaseInOut = 2,
    Hermite = 3,
}

impl RampInterp {
    pub fn from_menu(s: &str) -> Self {
        match s {
            "step" => Self::Step,
            "easeineaseout" => Self::EaseInOut,
            "hermite" => Self::Hermite,
            _ => Self::Linear,
        }
    }
}

fn parse_f(s: &str) -> Option<f32> {
    s.trim().parse::<f32>().ok()
}

fn parse_cell_f(s: &str) -> f32 {
    parse_f(s).unwrap_or(0.0)
}

fn is_header_row(row: &[String]) -> bool {
    let Some(first) = row.first() else {
        return true;
    };
    parse_f(first).is_none()
}

/// Table rows: `position r g b a` (0..1). Skips non-numeric first cells.
pub fn keyframes_from_rows(rows: &[Vec<String>]) -> Vec<RampKeyframe> {
    let mut keys = Vec::new();
    for row in rows {
        if row.len() < 5 {
            continue;
        }
        if is_header_row(row) {
            continue;
        }
        let Some(pos) = parse_f(&row[0]) else {
            continue;
        };
        keys.push(RampKeyframe {
            pos,
            rgba: [
                parse_cell_f(&row[1]),
                parse_cell_f(&row[2]),
                parse_cell_f(&row[3]),
                parse_cell_f(&row[4]),
            ],
        });
    }
    keys.sort_by(|a, b| {
        a.pos
            .partial_cmp(&b.pos)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    keys.dedup_by(|a, b| (a.pos - b.pos).abs() < 1e-6);
    keys
}

pub fn default_keyframes(node: &Node, eval: &EvalContext) -> Vec<RampKeyframe> {
    let c1 = node
        .param("color1")
        .map(|p| p.eval(eval).as_vec4_f32())
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let c2 = node
        .param("color2")
        .map(|p| p.eval(eval).as_vec4_f32())
        .unwrap_or([1.0, 1.0, 1.0, 1.0]);
    vec![
        RampKeyframe { pos: 0.0, rgba: c1 },
        RampKeyframe { pos: 1.0, rgba: c2 },
    ]
}

pub fn gather_keyframes(node: &Node, eval: &EvalContext, node_path: &str) -> Vec<RampKeyframe> {
    let dat_ref = node
        .param("dat")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default();
    if !dat_ref.trim().is_empty() {
        if let Some(net) = eval.channels {
            if let Some(path) = net.resolve_operator_path(node_path, dat_ref.trim()) {
                if let Some(rows) = net.dat_rows(&path) {
                    let parsed = keyframes_from_rows(&rows);
                    if parsed.len() >= 2 {
                        return parsed;
                    }
                    if parsed.len() == 1 {
                        let mut keys = default_keyframes(node, eval);
                        keys[0] = parsed[0].clone();
                        return keys;
                    }
                }
            }
        }
    }
    default_keyframes(node, eval)
}

pub fn revision(keys: &[RampKeyframe], interp: RampInterp, tension: f32) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    keys.len().hash(&mut h);
    for k in keys {
        k.pos.to_bits().hash(&mut h);
        for v in k.rgba {
            v.to_bits().hash(&mut h);
        }
    }
    (interp as u8).hash(&mut h);
    tension.to_bits().hash(&mut h);
    h.finish()
}

fn segment_t(u: f32, a: &RampKeyframe, b: &RampKeyframe) -> f32 {
    let span = (b.pos - a.pos).max(1e-6);
    ((u - a.pos) / span).clamp(0.0, 1.0)
}

fn interp_scalar(t: f32, mode: RampInterp) -> f32 {
    match mode {
        RampInterp::Step => {
            if t >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
        RampInterp::Linear => t,
        RampInterp::EaseInOut => t * t * (3.0 - 2.0 * t),
        RampInterp::Hermite => t,
    }
}

fn catmull_component(values: &[f32], i: usize, t: f32) -> f32 {
    let n = values.len();
    let i0 = i.saturating_sub(1);
    let i1 = i;
    let i2 = (i + 1).min(n - 1);
    let i3 = (i + 2).min(n - 1);
    let p0 = values[i0];
    let p1 = values[i1];
    let p2 = values[i2];
    let p3 = values[i3];
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

fn eval_at(keys: &[RampKeyframe], u: f32, mode: RampInterp, tension: f32) -> [f32; 4] {
    if keys.is_empty() {
        return [0.0; 4];
    }
    if keys.len() == 1 {
        return keys[0].rgba;
    }
    if u <= keys[0].pos {
        return keys[0].rgba;
    }
    if u >= keys[keys.len() - 1].pos {
        return keys[keys.len() - 1].rgba;
    }
    let seg = keys
        .windows(2)
        .position(|w| u >= w[0].pos && u <= w[1].pos)
        .unwrap_or(0);
    let a = &keys[seg];
    let b = &keys[seg + 1];
    let local = segment_t(u, a, b);
    match mode {
        RampInterp::Step => {
            if local >= 1.0 {
                b.rgba
            } else {
                a.rgba
            }
        }
        RampInterp::Hermite => {
            let mut out = [0.0; 4];
            let s = tension.clamp(0.0, 1.0);
            for (ch, slot) in out.iter_mut().enumerate() {
                let vals: Vec<f32> = keys.iter().map(|k| k.rgba[ch]).collect();
                let curved = catmull_component(&vals, seg, local);
                let linear = a.rgba[ch] + (b.rgba[ch] - a.rgba[ch]) * local;
                *slot = linear + (curved - linear) * s;
            }
            out
        }
        _ => {
            let t = interp_scalar(local, mode);
            [
                a.rgba[0] + (b.rgba[0] - a.rgba[0]) * t,
                a.rgba[1] + (b.rgba[1] - a.rgba[1]) * t,
                a.rgba[2] + (b.rgba[2] - a.rgba[2]) * t,
                a.rgba[3] + (b.rgba[3] - a.rgba[3]) * t,
            ]
        }
    }
}

pub fn bake_lut(keys: &[RampKeyframe], mode: RampInterp, tension: f32) -> Vec<u8> {
    let n = LUT_WIDTH as usize;
    let mut out = vec![0u8; n * 4];
    for i in 0..n {
        let u = if n <= 1 {
            0.0
        } else {
            i as f32 / (n - 1) as f32
        };
        let c = eval_at(keys, u, mode, tension);
        let base = i * 4;
        for (j, v) in c.iter().enumerate() {
            out[base + j] = (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        }
    }
    out
}

pub fn interp_from_node(node: &Node, eval: &EvalContext) -> RampInterp {
    let s = node
        .param("interp")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_else(|| "linear".into());
    RampInterp::from_menu(&s)
}

pub fn tension_from_node(node: &Node, eval: &EvalContext) -> f32 {
    node.param("tension")
        .map(|p| p.eval(eval).as_f32())
        .unwrap_or(0.0)
}

#[cfg(test)]
#[path = "tests/ramp__tests.rs"]
mod tests;
