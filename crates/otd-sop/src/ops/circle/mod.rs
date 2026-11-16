
use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "radius" => Param::new(Value::Vec2([1.0, 1.0])).with_label("Radius"),
        "divisions" => Param::int(32).with_label("Divisions").with_range(3.0, 512.0),
        "arc" => Param::float(360.0).with_label("Arc (degrees)").with_range(0.0, 360.0),
        "fill" => Param::bool(true).with_label("Fill"),
        "center" => Param::xyz([0.0, 0.0, 0.0]).with_label("Center"),
    }
}

/// A disc, a ring, or an arc of either. Unfilled it is a line primitive,
/// which is what makes it useful as a path for a Copy SOP to walk.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let r = c.v4("radius");
    let n = c.i("divisions").clamp(3, 512) as usize;
    let arc = c.f("arc").clamp(0.0, 360.0).to_radians();
    let closed = arc >= std::f32::consts::TAU - 1e-4;
    let o = c.v3("center");
    let fill = c.b("fill");

    let steps = if closed { n } else { n + 1 };
    let mut points = Vec::with_capacity(steps + 1);
    if fill {
        points.push(Point {
            position: o,
            normal: [0.0, 0.0, 1.0],
            uv: [0.5, 0.5],
            color: [1.0; 4],
        });
    }
    for i in 0..steps {
        let a = i as f32 / n as f32 * arc;
        points.push(Point {
            position: [o[0] + a.cos() * r[0], o[1] + a.sin() * r[1], o[2]],
            normal: [0.0, 0.0, 1.0],
            uv: [a.cos() * 0.5 + 0.5, a.sin() * 0.5 + 0.5],
            color: [1.0; 4],
        });
    }

    if !fill {
        return Geometry {
            points,
            indices: Vec::new(),
            topology: Topology::Lines,
            ..Default::default()
        };
    }
    let mut indices = Vec::with_capacity(steps * 3);
    for i in 0..steps {
        let a = 1 + i as u32;
        let b = if closed {
            1 + ((i + 1) % steps) as u32
        } else if i + 1 < steps {
            a + 1
        } else {
            continue;
        };
        indices.extend([0, a, b]);
    }
    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}
pub const TYPE_NAME: &str = "circleSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Circle",
        &[],
        "A disc, ring or arc — filled, or a line to copy along.",
        params,
        cook,
    )
}
