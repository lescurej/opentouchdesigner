use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "mathTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "operation" => Param::menu(
            "add",
            &["add", "subtract", "multiply", "divide", "minimum", "maximum", "difference", "power"],
        ).with_label("Operation"),
        "gain1" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Input 1 Gain"),
        "gain2" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Input 2 Gain"),
        "offset" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Offset"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "operation"), 0.0, 0.0, 0.0],
        v4(n, c, "gain1"),
        v4(n, c, "gain2"),
        v4(n, c, "offset"),
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Math",
            family: Family::Top,
            inputs: &["a", "b"],
            summary: "Arithmetic on two inputs, per channel.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/math.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
