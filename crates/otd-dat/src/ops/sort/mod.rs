use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "column" => Param::str("0").with_label("Column (name or index)"),
        "order" => Param::menu("ascending", &["ascending", "descending"]).with_label("Order"),
        "numeric" => Param::bool(true).with_label("Compare As Numbers"),
        "header" => Param::bool(true).with_label("First Row Is A Header"),
    }
}

fn column_index(rows: &[Vec<String>], spec: &str, header: bool) -> usize {
    if let Ok(i) = spec.trim().parse::<usize>() {
        return i;
    }
    if header {
        if let Some(head) = rows.first() {
            if let Some(i) = head.iter().position(|c| c == spec.trim()) {
                return i;
            }
        }
    }
    0
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let header = c.b("header");
    let col = column_index(&input.rows, &c.s("column"), header);
    let numeric = c.b("numeric");
    let descending = c.menu("order") == 1;

    let mut rows = input.rows.clone();
    let head = if header && !rows.is_empty() {
        Some(rows.remove(0))
    } else {
        None
    };
    rows.sort_by(|a, b| {
        let (x, y) = (
            a.get(col).map(|s| s.as_str()).unwrap_or(""),
            b.get(col).map(|s| s.as_str()).unwrap_or(""),
        );
        let ord = if numeric {
            // A cell that is not a number sorts as if it were zero rather
            // than falling back to string order for that one row, which
            // would interleave the two orderings unpredictably.
            let (x, y) = (
                x.trim().parse::<f64>().unwrap_or(0.0),
                y.trim().parse::<f64>().unwrap_or(0.0),
            );
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            x.cmp(y)
        };
        if descending { ord.reverse() } else { ord }
    });
    if let Some(head) = head {
        rows.insert(0, head);
    }
    DatData::table(rows)
}

pub const TYPE_NAME: &str = "sortDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Sort",
        &["in"],
        "Sort rows.",
        params,
        cook,
    )
}
