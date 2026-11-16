
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point, Topology};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "from" => Param::xyz([-1.0, 0.0, 0.0]).with_label("From"),
        "to" => Param::xyz([1.0, 0.0, 0.0]).with_label("To"),
        "points" => Param::int(2).with_label("Points").with_range(2.0, 4096.0),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let a = c.v3("from");
    let b = c.v3("to");
    let n = c.i("points").clamp(2, 4096) as usize;
    let points = (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            Point {
                position: [
                    a[0] + (b[0] - a[0]) * t,
                    a[1] + (b[1] - a[1]) * t,
                    a[2] + (b[2] - a[2]) * t,
                ],
                uv: [t, 0.0],
                ..Default::default()
            }
        })
        .collect();
    Geometry {
        points,
        indices: Vec::new(),
        topology: Topology::Lines,
        ..Default::default()
    }
}
pub const TYPE_NAME: &str = "lineSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Line",
        &[],
        "A run of points between two positions.",
        params,
        cook,
    )
}
