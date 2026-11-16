use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "cacheTOP";
pub const CACHE: &str = TYPE_NAME;

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "active" => Param::bool(true).with_label("Active"),
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
            label: "Cache",
            family: Family::Top,
            inputs: &["in"],
            summary: "Holds the last frame it saw when Active is off.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Cache,
    }
}
