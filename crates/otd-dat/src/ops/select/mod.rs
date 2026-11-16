use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "rows" => Param::str("*").with_label("Rows (names, indices or *)"),
        "cols" => Param::str("*").with_label("Columns"),
        "byname" => Param::bool(true).with_label("First Row/Column Are Names"),
    }
}

fn dat_gather_inputs(c: &DatCtx) -> DatData {
    let mut rows = Vec::new();
    for i in 0..c.inputs.len() {
        let inp = c.input(i);
        if inp.num_rows() > 0 {
            rows.extend(inp.rows.iter().cloned());
        }
    }
    DatData::table(rows)
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let input = dat_gather_inputs(c);
    let by_name = c.b("byname");

    let col_headings: Vec<String> = if by_name {
        input.rows.first().cloned().unwrap_or_default()
    } else {
        Vec::new()
    };
    let row_headings: Vec<String> = if by_name {
        input
            .rows
            .iter()
            .map(|r| r.first().cloned().unwrap_or_default())
            .collect()
    } else {
        Vec::new()
    };

    let rows = selection(&c.s("rows"), &row_headings, input.num_rows());
    let cols = selection(&c.s("cols"), &col_headings, input.num_cols());

    let picked = rows
        .iter()
        .map(|r| {
            cols.iter()
                .map(|col| input.cell(*r, *col).to_string())
                .collect()
        })
        .collect();
    DatData::table(picked)
}

// ---------------------------------------------------------------- merge
pub const TYPE_NAME: &str = "selectDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Select",
        &["in"],
        "Select rows/columns.",
        params,
        cook,
    )
}
