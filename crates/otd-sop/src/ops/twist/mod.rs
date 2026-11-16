
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "operation" => Param::menu("twist", &["twist", "bend", "taper"]).with_label("Operation"),
        "axis" => Param::menu("y", &["x", "y", "z"]).with_label("Primary Axis"),
        "strength" => Param::float(90.0).with_label("Strength (deg, or scale for taper)").with_range(-720.0, 720.0),
        "length" => Param::float(2.0).with_label("Deform Length").with_range(0.01, 16.0),
    }
}

/// Twist, bend or taper along one axis, proportional to the distance from
/// the origin along it. Normals are rotated with twist and bend; taper leaves
/// them, so follow it with a Facet SOP when lighting matters.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let op = c.menu("operation");
    let a = c.menu("axis");
    // (axis, a perpendicular) — bend curls the axis toward the perpendicular.
    let (b, p) = ((a + 1) % 3, (a + 2) % 3);
    let strength = c.f("strength");
    let length = c.f("length").max(1e-4);
    let rot = |v: [f32; 3], i: usize, j: usize, ang: f32| {
        let (s, co) = ang.sin_cos();
        let mut out = v;
        out[i] = v[i] * co - v[j] * s;
        out[j] = v[i] * s + v[j] * co;
        out
    };
    c.input(0).map_points(|_, pt| {
        let t = pt.position[a] / length;
        match op {
            // Rotate the cross-section around the axis.
            0 => {
                let ang = (strength * t).to_radians();
                Point {
                    position: rot(pt.position, b, p, ang),
                    normal: rot(pt.normal, b, p, ang),
                    ..*pt
                }
            }
            // Wrap the axis onto an arc of radius length/strength, curling
            // toward the next axis; the origin stays put.
            1 => {
                let total = strength.to_radians();
                if total.abs() < 1e-6 {
                    return *pt;
                }
                let r = length / total;
                let ang = total * t;
                let w = r - pt.position[b];
                let mut q = pt.position;
                q[a] = w * ang.sin();
                q[b] = r - w * ang.cos();
                Point {
                    position: q,
                    normal: rot(pt.normal, a, b, ang),
                    ..*pt
                }
            }
            _ => {
                let k = 1.0 + strength / 100.0 * t;
                let mut q = pt.position;
                q[b] *= k;
                q[p] *= k;
                Point { position: q, ..*pt }
            }
        }
    })
}
pub const TYPE_NAME: &str = "twistSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Twist",
        &["in"],
        "Twist, bend or taper along one axis, growing with distance from the origin.",
        params,
        cook,
    )
}
