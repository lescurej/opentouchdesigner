use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        // The table's contents live in the project file, so a cue list is
        // versioned with the patch that uses it.
        "text" => Param::str("name\tvalue\n").with_label("Contents"),
        "delimiter" => Param::menu("tab", &["tab", "comma"]).with_label("Delimiter"),
    }
}

fn delimiter_of(c: &DatCtx) -> char {
    if c.menu("delimiter") == 1 { ',' } else { '\t' }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    DatData::from_delimited(&c.s("text"), delimiter_of(c))
}

// --------------------------------------------------------------- select
pub const TYPE_NAME: &str = "tableDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Table",
        &[],
        "A table of text, stored in the project file.",
        params,
        cook,
    )
}
