use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "selectTOP";

pub const SELECT: &str = TYPE_NAME;

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "top" => Param::str("").with_label("TOP"),
    })
}


fn pack(_n: &Node, _c: &EvalContext) -> PackedParams {
    [[0.0; 4]; 4]
}


pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Select",
            family: Family::Top,
            inputs: &[],
            summary: "This frame's output of another TOP, by path.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Referenced,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Select,
    }
}
