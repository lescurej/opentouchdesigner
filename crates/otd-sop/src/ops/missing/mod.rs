

use crate::Geometry;

use super::helpers::params_missing;
use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    c.input(0)
}
pub const TYPE_NAME: &str = "otd_core::MISSING_SOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Missing",
        &["in"],
        "Placeholder for an unknown or removed SOP.",
        params_missing,
        cook,
    )
}
