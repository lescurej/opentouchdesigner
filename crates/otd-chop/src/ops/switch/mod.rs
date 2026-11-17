use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::ChopData;

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "index" => Param::float(0.0).with_label("Index").with_range(0.0, 32.0),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let n = c.inputs.len().max(1);
    let index = (c.f("index").round() as i64).clamp(0, n as i64 - 1) as usize;
    c.input(index).clone()
}




pub const TYPE_NAME: &str = "switchCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Switch",
                &["0", "1"],
                "Choose one input by index.",
                params,
                cook,
            )
}
