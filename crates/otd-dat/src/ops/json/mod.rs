use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;
use crate::ops::DatCtx;

use super::{DatSpec, spec as dat_spec};

pub const TYPE_NAME: &str = "jsonDAT";



/// Flatten JSON into `path`/`value` rows.
///
/// A table is what the rest of the network can use; keeping the flattened
/// path as the key means a Select DAT can pull one field out by name.
fn flatten(prefix: &str, v: &serde_json::Value, out: &mut Vec<Vec<String>>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                flatten(&format!("{prefix}/{k}"), v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                flatten(&format!("{prefix}/{i}"), v, out);
            }
        }
        serde_json::Value::Null => out.push(vec![prefix.to_string(), String::new()]),
        serde_json::Value::Bool(b) => out.push(vec![prefix.to_string(), b.to_string()]),
        serde_json::Value::Number(n) => out.push(vec![prefix.to_string(), n.to_string()]),
        serde_json::Value::String(s) => out.push(vec![prefix.to_string(), s.clone()]),
    }
}


pub(crate) fn params_json() -> IndexMap<String, Param> {
    params! {
        "pointer" => Param::str("").with_label("JSON Pointer (e.g. /items/0)"),
    }
}


pub(crate) fn cook_json(c: &mut DatCtx) -> DatData {
    let text = c.input(0).as_text();
    if text.trim().is_empty() {
        return DatData::table(vec![vec!["path".into(), "value".into()]]);
    }
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            c.error = Some(format!("JSON: {e}"));
            return DatData::table(vec![vec!["path".into(), "value".into()]]);
        }
    };
    let pointer = c.s("pointer");
    let root = if pointer.trim().is_empty() {
        &parsed
    } else {
        match parsed.pointer(pointer.trim()) {
            Some(v) => v,
            None => {
                c.error = Some(format!("no JSON at `{}`", pointer.trim()));
                return DatData::table(vec![vec!["path".into(), "value".into()]]);
            }
        }
    };
    let mut rows = vec![vec!["path".to_string(), "value".to_string()]];
    flatten("", root, &mut rows);
    DatData::table(rows)
}

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "JSON",
        &["in"],
        "Parse JSON text into a path/value table.",
        params_json,
        cook_json,
    )
}
