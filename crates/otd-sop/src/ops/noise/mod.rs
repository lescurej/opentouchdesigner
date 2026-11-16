
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "amplitude" => Param::float(0.2).with_label("Amplitude").with_range(0.0, 4.0),
        "period" => Param::float(1.0).with_label("Period").with_range(0.01, 10.0),
        "offset" => Param::xyz([0.0, 0.0, 0.0]).with_label("Offset"),
        "along" => Param::menu("normal", &["normal", "xyz"]).with_label("Displace Along"),
    }
}

fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x9e3779b9)
        ^ (y as u32).wrapping_mul(0x85ebca6b)
        ^ (z as u32).wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb352d);
    h ^= h >> 15;
    (h as f32 / u32::MAX as f32) * 2.0 - 1.0
}

fn value_noise(p: [f32; 3]) -> f32 {
    let i = [p[0].floor(), p[1].floor(), p[2].floor()];
    let f = [p[0] - i[0], p[1] - i[1], p[2] - i[2]];
    let u = [
        f[0] * f[0] * (3.0 - 2.0 * f[0]),
        f[1] * f[1] * (3.0 - 2.0 * f[1]),
        f[2] * f[2] * (3.0 - 2.0 * f[2]),
    ];
    let mut acc = 0.0;
    for (dz, wz) in [(0, 1.0 - u[2]), (1, u[2])] {
        for (dy, wy) in [(0, 1.0 - u[1]), (1, u[1])] {
            for (dx, wx) in [(0, 1.0 - u[0]), (1, u[0])] {
                let h = hash3(i[0] as i32 + dx, i[1] as i32 + dy, i[2] as i32 + dz);
                acc += h * wx * wy * wz;
            }
        }
    }
    acc
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let amp = c.f("amplitude");
    let period = c.f("period").max(1e-4);
    let off = c.v3("offset");
    let along_normal = c.menu("along") == 0;

    c.input(0).map_points(|_, p| {
        let sample = [
            p.position[0] / period + off[0],
            p.position[1] / period + off[1],
            p.position[2] / period + off[2],
        ];
        let d = if along_normal {
            let n = value_noise(sample) * amp;
            [p.normal[0] * n, p.normal[1] * n, p.normal[2] * n]
        } else {
            [
                value_noise(sample) * amp,
                value_noise([sample[0] + 17.3, sample[1] + 5.1, sample[2] + 91.7]) * amp,
                value_noise([sample[0] + 41.9, sample[1] + 63.2, sample[2] + 7.4]) * amp,
            ]
        };
        Point {
            position: [
                p.position[0] + d[0],
                p.position[1] + d[1],
                p.position[2] + d[2],
            ],
            ..*p
        }
    })
}
pub const TYPE_NAME: &str = "noiseSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Noise",
        &["in"],
        "Displace points by value noise, along their normals or freely.",
        params,
        cook,
    )
}
