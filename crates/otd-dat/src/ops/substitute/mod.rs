use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "template" => Param::str("").with_label("Template"),
        "table" => Param::bool(false).with_label("Substitute Every Cell Instead"),
    }
}

/// Fill `$name` placeholders from a two-column lookup table.
///
/// This is the operator that turns a cue table into a string somebody else
/// can read — an OSC address, a UDP payload, a line of a show report — without
/// a script. The input's first column names, the second supplies.
pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let substitute = |text: &str| -> String {
        let mut out = text.to_string();
        for row in &input.rows {
            let (Some(key), Some(value)) = (row.first(), row.get(1)) else {
                continue;
            };
            if key.trim().is_empty() {
                continue;
            }
            out = out.replace(&format!("${key}"), value);
        }
        out
    };

    if !c.b("table") {
        return DatData::text(substitute(&c.s("template")));
    }
    DatData::table(
        input
            .rows
            .iter()
            .map(|r| r.iter().map(|cell| substitute(cell)).collect())
            .collect(),
    )
}

// --------------------------------------------------------------- convert
pub const TYPE_NAME: &str = "substituteDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Substitute",
        &["in"],
        "Fill $name placeholders.",
        params,
        cook,
    )
}
