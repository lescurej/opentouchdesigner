use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "constantTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "color" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Color"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [v4(n, c, "color"), [0.0; 4], [0.0; 4], [0.0; 4]]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Constant",
            family: Family::Top,
            inputs: &[],
            summary: "A flat colour at a chosen resolution.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/constant.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}