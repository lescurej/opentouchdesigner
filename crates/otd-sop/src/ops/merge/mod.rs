

use crate::Geometry;

use super::helpers::no_params;
use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    c.input(0).merged(&c.input(1))
}
pub const TYPE_NAME: &str = "mergeSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Merge",
        &["a", "b"],
        "Combine two pieces of geometry.",
        no_params,
        cook,
    )
}
