use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "feedbackTOP";
pub const FEEDBACK: &str = TYPE_NAME;

fn params() -> otd_core::indexmap::IndexMap<String, Param> {
    merge_common(params! {
        "target" => Param::str("").with_label("Target TOP"),
    })
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Feedback",
            family: Family::Top,
            inputs: &[],
            summary: "Last frame's output of the Target TOP.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Referenced,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::Feedback,
    }
}