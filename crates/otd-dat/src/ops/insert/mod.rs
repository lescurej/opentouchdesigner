use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "direction" => Param::menu("rows", &["rows", "columns"]).with_label("Insert"),
        "index" => Param::int(0).with_label("At Index").with_range(0.0, 10000.0),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let base = c.input(0);
    let insert = c.input(1);
    let index = c.val("index").as_i64().max(0) as usize;
    if c.menu("direction") == 1 {
        let mut rows: Vec<Vec<String>> = base.rows.clone();
        for (r, ins_row) in insert.rows.iter().enumerate() {
            let target = index + r;
            while rows.len() <= target {
                rows.push(Vec::new());
            }
            let row = &mut rows[target];
            let mut merged = row.clone();
            merged.extend(ins_row.iter().cloned());
            *row = merged;
        }
        return DatData::table(rows);
    }
    let mut rows = base.rows.clone();
    if index > rows.len() {
        rows.resize(index, Vec::new());
    }
    for (i, row) in insert.rows.iter().enumerate() {
        rows.insert(index + i, row.clone());
    }
    DatData::table(rows)
}
pub const TYPE_NAME: &str = "insertDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Insert",
        &["base", "insert"],
        "Insert rows/columns.",
        params,
        cook,
    )
}
