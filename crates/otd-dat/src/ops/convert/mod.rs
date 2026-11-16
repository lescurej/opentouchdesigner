use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "to" => Param::menu("table", &["table", "text"]).with_label("Convert To"),
        "delimiter" => Param::menu("tab", &["tab", "comma"]).with_label("Delimiter"),
    }
}

/// Between the two shapes a DAT can have.
///
/// They are one representation internally — a text DAT is a table with one
/// cell — but the operators reading them are not interchangeable, so the
/// conversion has to be something you can put in a patch rather than a flag
/// hidden on every node.
pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let delim = if c.menu("delimiter") == 1 { ',' } else { '\t' };
    if c.menu("to") == 1 {
        let text = input
            .rows
            .iter()
            .map(|r| r.join(&delim.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        return DatData::text(text);
    }
    DatData::from_delimited(&input.as_text(), delim)
}

// ------------------------------------------------------- CHOP to DAT
pub const TYPE_NAME: &str = "convertDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Convert",
        &["in"],
        "Table/text convert.",
        params,
        cook,
    )
}
