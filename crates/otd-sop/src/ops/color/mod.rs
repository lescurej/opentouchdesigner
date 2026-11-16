
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::{Geometry, Point};

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "color" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color"),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let rgba = c.v4("color");
    c.input(0).map_points(|_, p| Point { color: rgba, ..*p })
}
pub const TYPE_NAME: &str = "colorSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Color",
        &["in"],
        "Set the colour carried by every point.",
        params,
        cook,
    )
}
