
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "radius1" => Param::float(1.0).with_label("Outer Radius").with_range(0.0, 8.0),
        "radius2" => Param::float(0.3).with_label("Inner Radius").with_range(0.0, 8.0),
        "rows" => Param::int(24).with_label("Rows").with_range(3.0, 256.0),
        "columns" => Param::int(24).with_label("Columns").with_range(3.0, 256.0),
        "center" => Param::xyz([0.0, 0.0, 0.0]).with_label("Center"),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let (big, small) = (c.f("radius1"), c.f("radius2"));
    let rows = c.i("rows").clamp(3, 256) as usize;
    let cols = c.i("columns").clamp(3, 256) as usize;
    let o = c.v3("center");

    let mut points = Vec::with_capacity((rows + 1) * (cols + 1));
    for row in 0..=rows {
        let v = row as f32 / rows as f32;
        let phi = v * std::f32::consts::TAU;
        for col in 0..=cols {
            let u = col as f32 / cols as f32;
            let theta = u * std::f32::consts::TAU;
            // The normal points away from the centre of the tube, which is
            // the circle of radius `big` — not away from the origin.
            let n = [phi.cos() * theta.cos(), phi.sin(), phi.cos() * theta.sin()];
            let r = big + small * phi.cos();
            points.push(Point {
                position: [
                    o[0] + theta.cos() * r,
                    o[1] + small * phi.sin(),
                    o[2] + theta.sin() * r,
                ],
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
            // As with the Tube: rows run the opposite way to the Sphere's, so
            // they need the opposite index order to wind outwards.
            indices.extend([a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}
pub const TYPE_NAME: &str = "torusSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Torus",
        &[],
        "A torus.",
        params,
        cook,
    )
}
