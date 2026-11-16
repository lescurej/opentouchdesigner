
use crate::DatData;

use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let cols = input.num_cols();
    let rows = (0..cols)
        .map(|col| {
            input
                .rows
                .iter()
                .map(|r| r.get(col).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    DatData::table(rows)
}

pub const TYPE_NAME: &str = "transposeDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Transpose",
        &["in"],
        "Swap rows and columns.",
        no_params,
        cook,
    )
}
