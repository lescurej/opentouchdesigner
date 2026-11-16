
use crate::DatData;

use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    c.input(0)
}
pub const TYPE_NAME: &str = "otd_core::MISSING_DAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Missing",
        &["in"],
        "Placeholder DAT.",
        params_missing,
        cook,
    )
}
