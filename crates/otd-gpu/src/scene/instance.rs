use otd_core::{EvalContext, Node};

use super::helpers::{b, f, s};

/// One instance, in the layout the instance vertex buffer wants.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub translate: [f32; 3],
    pub scale: [f32; 3],
    pub rotate: [f32; 3],
    pub color: [f32; 4],
}

impl Default for Instance {
    fn default() -> Self {
        Instance {
            translate: [0.0; 3],
            scale: [1.0; 3],
            rotate: [0.0; 3],
            color: [1.0; 4],
        }
    }
}

use super::Scene;

/// Build the instance list for a Geometry COMP.
///
/// Returns a single identity instance when instancing is off, so the render
/// path has one shape rather than two.
pub fn instances(node: &Node, ctx: &EvalContext, scene: &dyn Scene) -> Vec<Instance> {
    if !b(node, ctx, "instancing") {
        return vec![Instance::default()];
    }
    let path = s(node, ctx, "instancechop");
    let Some(channels) = scene.channels(path.trim()) else {
        return vec![Instance::default()];
    };

    let pick = |name: &str| -> Option<&Vec<f32>> {
        if name.trim().is_empty() {
            return None;
        }
        channels
            .iter()
            .find(|(n, _)| n == name.trim())
            .map(|(_, v)| v)
    };

    // Each sample is an instance, so the count is the longest channel that is
    // actually referenced.
    let names = [
        s(node, ctx, "tx"),
        s(node, ctx, "ty"),
        s(node, ctx, "tz"),
        s(node, ctx, "sx"),
        s(node, ctx, "sy"),
        s(node, ctx, "sz"),
        s(node, ctx, "rx"),
        s(node, ctx, "ry"),
        s(node, ctx, "rz"),
        s(node, ctx, "cr"),
        s(node, ctx, "cg"),
        s(node, ctx, "cb"),
        s(node, ctx, "ca"),
    ];
    let count = names
        .iter()
        .filter_map(|n| pick(n))
        .map(|c| c.len())
        .max()
        .unwrap_or(0);
    if count == 0 {
        return vec![Instance::default()];
    }

    let sample = |channel: Option<&Vec<f32>>, i: usize, default: f32| -> f32 {
        match channel {
            // A single-sample channel broadcasts to every instance, which is
            // what makes "one LFO scales all of them" work.
            Some(c) if !c.is_empty() => c[i.min(c.len() - 1)],
            _ => default,
        }
    };
    let chans: Vec<Option<&Vec<f32>>> = names.iter().map(|n| pick(n)).collect();
    let uniform_scale = f(node, ctx, "instancescale");

    (0..count.min(1 << 20))
        .map(|i| Instance {
            translate: [
                sample(chans[0], i, 0.0),
                sample(chans[1], i, 0.0),
                sample(chans[2], i, 0.0),
            ],
            scale: [
                sample(chans[3], i, 1.0) * uniform_scale,
                sample(chans[4], i, 1.0) * uniform_scale,
                sample(chans[5], i, 1.0) * uniform_scale,
            ],
            rotate: [
                sample(chans[6], i, 0.0),
                sample(chans[7], i, 0.0),
                sample(chans[8], i, 0.0),
            ],
            color: [
                sample(chans[9], i, 1.0),
                sample(chans[10], i, 1.0),
                sample(chans[11], i, 1.0),
                sample(chans[12], i, 1.0),
            ],
        })
        .collect()
}
