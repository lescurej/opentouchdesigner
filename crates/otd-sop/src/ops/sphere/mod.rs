
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "radius" => Param::float(0.5).with_label("Radius").with_range(0.01, 10.0),
        "rows" => Param::int(16).with_label("Rows").with_range(3.0, 128.0),
        "columns" => Param::int(24).with_label("Columns").with_range(3.0, 256.0),
        "center" => Param::xyz([0.0, 0.0, 0.0]).with_label("Center"),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let r = c.f("radius");
    let rows = c.i("rows").clamp(3, 128) as usize;
    let cols = c.i("columns").clamp(3, 256) as usize;
    let o = c.v3("center");

    let mut points = Vec::with_capacity((rows + 1) * (cols + 1));
    for row in 0..=rows {
        let v = row as f32 / rows as f32;
        let phi = v * std::f32::consts::PI;
        for col in 0..=cols {
            let u = col as f32 / cols as f32;
            let theta = u * std::f32::consts::TAU;
            let n = [phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin()];
            points.push(Point {
                position: [o[0] + n[0] * r, o[1] + n[1] * r, o[2] + n[2] * r],
                normal: n,
                uv: [u, v],
                color: [1.0; 4],
            });
        }
    }

    let stride = cols + 1;
    let mut indices = Vec::with_capacity(rows * cols * 6);
    for row in 0..rows {
        for col in 0..cols {
            let a = (row * stride + col) as u32;
            let b = a + stride as u32;
            // Counter-clockwise seen from outside, so the winding agrees with
            // the vertex normals and back-face culling keeps the right half.
            indices.extend([a, a + 1, b, a + 1, b + 1, b]);
        }
    }
    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}
pub const TYPE_NAME: &str = "sphereSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Sphere",
        &[],
        "A UV sphere.",
        params,
        cook,
    )
}
