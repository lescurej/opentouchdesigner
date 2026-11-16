
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point};

use super::transform::rotate_xyz;
use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "count" => Param::int(3).with_label("Copies").with_range(1.0, 512.0),
        "translate" => Param::xyz([1.0, 0.0, 0.0]).with_label("Translate Each"),
        "rotate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Rotate Each (deg)"),
        "scale" => Param::xyz([1.0, 1.0, 1.0]).with_label("Scale Each"),
    }
}

/// Stamp copies out with a compounding transform.
///
/// This is the CPU version of what instancing does on the GPU; it exists for
/// the familiar mental model and for small counts. Anything large should be
/// instanced instead, which is why the Geometry COMP takes its transforms
/// from a CHOP rather than from geometry duplicated here.
pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let n = c.i("count").clamp(1, 512) as usize;
    let t = c.v3("translate");
    let r = c.v3("rotate");
    let s = c.v3("scale");
    let source = c.input(0);

    let mut out = Geometry {
        topology: source.topology,
        ..Default::default()
    };
    for copy in 0..n {
        let k = copy as f32;
        let scale = [s[0].powf(k), s[1].powf(k), s[2].powf(k)];
        let rot = [r[0] * k, r[1] * k, r[2] * k];
        let offset = [t[0] * k, t[1] * k, t[2] * k];
        let stamped = source.map_points(|_, p| {
            let scaled = [
                p.position[0] * scale[0],
                p.position[1] * scale[1],
                p.position[2] * scale[2],
            ];
            let rotated = rotate_xyz(scaled, rot);
            Point {
                position: [
                    rotated[0] + offset[0],
                    rotated[1] + offset[1],
                    rotated[2] + offset[2],
                ],
                normal: rotate_xyz(p.normal, rot),
                ..*p
            }
        });
        out = out.merged(&stamped);
    }
    out
}
pub const TYPE_NAME: &str = "copySOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Copy",
        &["in"],
        "Stamp copies with a compounding transform.",
        params,
        cook,
    )
}
