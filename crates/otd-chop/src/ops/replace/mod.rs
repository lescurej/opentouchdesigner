use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::ChopData;
use crate::ops::ChopCtx;
use crate::ops::helpers::map_samples;

use super::ChopSpec;

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "value" => Param::float(0.0).with_label("Value"),
        "replace" => Param::float(1.0).with_label("Replace With"),
        "tolerance" => Param::float(0.0).with_label("Tolerance").with_range(0.0, 10.0),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let find = c.f("value");
    let repl = c.f("replace");
    let tol = c.f("tolerance").max(0.0);
    map_samples(
        &input,
        |_, _, v| {
            if (v - find).abs() <= tol { repl } else { v }
        },
    )
}

pub const TYPE_NAME: &str = "replaceCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Replace",
        &["in"],
        "Replace samples that match a value.",
        params,
        cook,
    )
}
