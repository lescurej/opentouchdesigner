use crate::data::ChopData;

use super::{spec as chop_spec, ChopCtx, ChopSpec, NULL};
use super::helpers::no_params;

pub const TYPE_NAME: &str = NULL;

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    c.input(0).clone()
}

pub fn spec() -> ChopSpec {
    chop_spec(
        TYPE_NAME,
        "Null",
        &["in"],
        "Pass-through. A stable name to export from.",
        no_params,
        cook,
    )
}