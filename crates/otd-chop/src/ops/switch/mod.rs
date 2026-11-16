use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::ChopData;

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "index" => Param::float(0.0).with_label("Index").with_range(0.0, 1.0),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let index = c.f("index");
    if index >= 0.5 {
        c.input(1).clone()
    } else {
        c.input(0).clone()
    }
}




pub const TYPE_NAME: &str = "switchCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Switch",
                &["0", "1"],
                "Choose one of two inputs.",
                params,
                cook,
            )
}
