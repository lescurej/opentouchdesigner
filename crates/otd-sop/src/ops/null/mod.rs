

use crate::Geometry;

use super::helpers::no_params;
use super::{SopCtx, SopSpec, spec as sop_spec};

pub(crate) fn cook(c: &mut SopCtx) -> Geometry {
    c.input(0)
}
pub const TYPE_NAME: &str = "nullSOP";

pub fn spec() -> SopSpec {
    sop_spec(
        TYPE_NAME,
        "Null",
        &["in"],
        "Pass-through. A stable name to reference.",
        no_params,
        cook,
    )
}
