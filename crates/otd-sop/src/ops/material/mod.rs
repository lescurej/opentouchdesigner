
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::Geometry;

use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "mat" => Param::str("").with_label("Material").as_path_ref(),
    }
}

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    let mut geo = c.input(0);
    geo.mat_path = c.s("mat");
    geo
}
pub const TYPE_NAME: &str = "materialSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Material",
        &["in"],
        "Assign a MAT to geometry.",
        params,
        cook,
    )
}
