use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "switchTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "index" => Param::float(0.0).with_label("Index").with_range(0.0, 31.0),
        "blend" => Param::bool(false).with_label("Blend"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [f(n, c, "index"), b(n, c, "blend"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Switch",
            family: Family::Top,
            inputs: &["in1", "in2", "in3", "in4"],
            summary: "Select one of its inputs by index, optionally blending.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/switch.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}