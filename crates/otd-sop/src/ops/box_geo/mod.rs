
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "size" => Param::xyz([1.0, 1.0, 1.0]).with_label("Size"),
        "center" => Param::xyz([0.0, 0.0, 0.0]).with_label("Center"),
    }
}

pub(crate) fn box_geometry(size: [f32; 3], center: [f32; 3]) -> Geometry {
    let (hx, hy, hz) = (size[0] * 0.5, size[1] * 0.5, size[2] * 0.5);
    let o = center;

    // (normal, u axis, v axis) per face.
    let faces: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
        ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
        ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
        ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ];

    let mut points = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    for (n, u, v) in faces {
        let base = points.len() as u32;
        let centre = [n[0] * hx, n[1] * hy, n[2] * hz];
        for (du, dv, uu, vv) in [
            (-1.0f32, -1.0f32, 0.0f32, 0.0f32),
            (1.0, -1.0, 1.0, 0.0),
            (1.0, 1.0, 1.0, 1.0),
            (-1.0, 1.0, 0.0, 1.0),
        ] {
            points.push(Point {
                position: [
                    o[0] + centre[0] + u[0] * du * hx + v[0] * dv * hx,
                    o[1] + centre[1] + u[1] * du * hy + v[1] * dv * hy,
                    o[2] + centre[2] + u[2] * du * hz + v[2] * dv * hz,
                ],
                normal: n,
                uv: [uu, vv],
                color: [1.0; 4],
            });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    Geometry {
        points,
        indices,
        topology: Topology::Triangles,
        ..Default::default()
    }
}

/// The six faces, each as two triangles with a flat normal.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    box_geometry(c.v3("size"), c.v3("center"))
}
pub const TYPE_NAME: &str = "boxSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Box",
        &[],
        "A box with flat-shaded faces.",
        params,
        cook,
    )
}
