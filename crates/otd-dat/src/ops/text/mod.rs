use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "text" => Param::str("").with_label("Text"),
        "language" => Param::menu("text", &["text", "python", "glsl"]).with_label("Language"),
        "extension" => Param::str("").with_label("Extension"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    DatData::text(c.s("text"))
}

pub const TYPE_NAME: &str = "textDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Text",
        &[],
        "A block of text.",
        params,
        cook,
    )
}
