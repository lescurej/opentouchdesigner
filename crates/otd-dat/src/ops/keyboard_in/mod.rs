use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "keys" => Param::str("").with_label("Keys (empty = all held keys)"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let mut rows = vec![vec!["key".into(), "down".into()]];
    let filter = c.s("keys");
    let held: Vec<String> = c.input_keys.map(|k| k.to_vec()).unwrap_or_default();
    let list: Vec<String> = if filter.trim().is_empty() {
        held.clone()
    } else {
        filter.split_whitespace().map(str::to_string).collect()
    };
    for key in list {
        let down = held.iter().any(|h| h.eq_ignore_ascii_case(&key));
        rows.push(vec![key, if down { "1" } else { "0" }.into()]);
    }
    DatData::table(rows)
}
pub const TYPE_NAME: &str = "keyboardinDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Keyboard In",
        &[],
        "Key table.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
