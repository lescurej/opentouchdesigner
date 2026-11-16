use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "how" => Param::menu("rows", &["rows", "columns"]).with_label("Merge By"),
    }
}

fn merge_dat_columns(a: &DatData, b: &DatData) -> DatData {
    let n = a.num_rows().max(b.num_rows());
    let rows = (0..n)
        .map(|r| {
            let mut row: Vec<String> = a.rows.get(r).cloned().unwrap_or_default();
            row.resize(a.num_cols(), String::new());
            row.extend(b.rows.get(r).cloned().unwrap_or_default());
            row
        })
        .collect();
    DatData::table(rows)
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let mut parts: Vec<DatData> = (0..c.inputs.len())
        .map(|i| c.input(i))
        .filter(|d| d.num_rows() > 0)
        .collect();
    if parts.is_empty() {
        return DatData::default();
    }
    if c.menu("how") == 1 {
        let mut out = parts.remove(0);
        for next in parts {
            out = merge_dat_columns(&out, &next);
        }
        out
    } else {
        let mut rows = Vec::new();
        for part in parts {
            rows.extend(part.rows.iter().cloned());
        }
        DatData::table(rows)
    }
}
pub const TYPE_NAME: &str = "mergeDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Merge",
        &["in"],
        "Merge DATs by rows or columns.",
        params,
        cook,
    )
}
