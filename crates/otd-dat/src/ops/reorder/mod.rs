use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "direction" => Param::menu("rows", &["rows", "columns"]).with_label("Reorder"),
        "order" => Param::str("*").with_label("Order (indices or names)"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let spec = c.s("order");
    if c.menu("direction") == 1 {
        let headings: Vec<String> = input.rows.first().cloned().unwrap_or_default();
        let cols = selection(&spec, &headings, input.num_cols());
        let rows = input
            .rows
            .iter()
            .map(|r| {
                cols.iter()
                    .map(|ci| r.get(*ci).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();
        return DatData::table(rows);
    }
    let row_headings: Vec<String> = input
        .rows
        .iter()
        .map(|r| r.first().cloned().unwrap_or_default())
        .collect();
    let rows_idx = selection(&spec, &row_headings, input.num_rows());
    let rows = rows_idx
        .iter()
        .map(|ri| input.rows.get(*ri).cloned().unwrap_or_default())
        .collect();
    DatData::table(rows)
}
pub const TYPE_NAME: &str = "reorderDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Reorder",
        &["in"],
        "Reorder rows/columns.",
        params,
        cook,
    )
}
