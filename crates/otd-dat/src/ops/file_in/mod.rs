use std::fs;
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "file" => Param::str("").with_label("File").as_file_ref(),
        "astable" => Param::bool(false).with_label("Read As Table"),
        "delimiter" => Param::menu("tab", &["tab", "comma"]).with_label("Delimiter"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let path = c.graph.resolve_external(&c.s("file"));
    if path.as_os_str().is_empty() {
        return DatData::text("");
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            c.error = Some(format!("file in: {e}"));
            return DatData::text("");
        }
    };
    if c.b("astable") {
        let delim = if c.menu("delimiter") == 1 { ',' } else { '\t' };
        return DatData::from_delimited(&text, delim);
    }
    DatData::text(text)
}
pub const TYPE_NAME: &str = "fileinDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "File In",
        &[],
        "Read text file.",
        params,
        cook,
    )
}
