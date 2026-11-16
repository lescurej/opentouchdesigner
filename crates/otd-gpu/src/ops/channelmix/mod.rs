use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use otd_core::Value;

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "channelmixTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "red" => Param::new(Value::Vec4([1.0, 0.0, 0.0, 0.0])).with_label("Red From RGBA"),
        "green" => Param::new(Value::Vec4([0.0, 1.0, 0.0, 0.0])).with_label("Green From RGBA"),
        "blue" => Param::new(Value::Vec4([0.0, 0.0, 1.0, 0.0])).with_label("Blue From RGBA"),
        "alpha" => Param::new(Value::Vec4([0.0, 0.0, 0.0, 1.0])).with_label("Alpha From RGBA"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        v4(n, c, "red"),
        v4(n, c, "green"),
        v4(n, c, "blue"),
        v4(n, c, "alpha"),
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Channel Mix",
            family: Family::Top,
            inputs: &["in"],
            summary: "Each output channel is a weighted sum of the input's four.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/channelmix.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
