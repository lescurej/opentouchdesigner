
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "translate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Translate"),
        "rotate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Rotate (deg)"),
        "scale" => Param::xyz([1.0, 1.0, 1.0]).with_label("Scale"),
    }
}

/// Rotate a vector by XYZ Euler angles in degrees.
pub fn rotate_xyz(v: [f32; 3], deg: [f32; 3]) -> [f32; 3] {
    let (rx, ry, rz) = (
        deg[0].to_radians(),
        deg[1].to_radians(),
        deg[2].to_radians(),
    );
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();
    // X, then Y, then Z.
    let (y, z) = (v[1] * cx - v[2] * sx, v[1] * sx + v[2] * cx);
    let (x, z) = (v[0] * cy + z * sy, -v[0] * sy + z * cy);
    let (x, y) = (x * cz - y * sz, x * sz + y * cz);
    [x, y, z]
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let t = c.v3("translate");
    let r = c.v3("rotate");
    let s = c.v3("scale");
    c.input(0).map_points(|_, p| {
        let scaled = [
            p.position[0] * s[0],
            p.position[1] * s[1],
            p.position[2] * s[2],
        ];
        let rotated = rotate_xyz(scaled, r);
        Point {
            position: [rotated[0] + t[0], rotated[1] + t[1], rotated[2] + t[2]],
            // Normals rotate but do not translate or scale.
            normal: rotate_xyz(p.normal, r),
            ..*p
        }
    })
}
pub const TYPE_NAME: &str = "transformSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Transform",
        &["in"],
        "Translate, rotate and scale points.",
        params,
        cook,
    )
}
