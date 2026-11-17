use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::ChopData;

use super::helpers::resolve_op_path;
use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "target" => Param::str("").with_label("Target CHOP"),
        "reset" => Param::bool(false).with_label("Reset"),
        "resetpulse" => Param::bool(false).with_label("Reset Pulse"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    if c.b("reset") || c.b("resetpulse") {
        c.clear_prev = true;
        return ChopData::empty();
    }
    let reference = c.s("target");
    if let Some(id) = resolve_op_path(c, &reference) {
        if let Some(target) = c.graph.resolve_output(id).filter(|t| *t != c.id) {
            c.capture = Some(target);
        }
    }
    c.prev_frame
        .as_ref()
        .map(|data| data.as_ref().clone())
        .unwrap_or_else(ChopData::empty)
}

pub const TYPE_NAME: &str = "feedbackCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
        TYPE_NAME,
        "Feedback",
        &[],
        "Last frame's channels of the Target CHOP.",
        params,
        cook,
    )
}
