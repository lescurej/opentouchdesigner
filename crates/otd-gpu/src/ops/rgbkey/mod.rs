use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "rgbkeyTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "color" => Param::rgba([0.0, 1.0, 0.0, 1.0]).with_label("Key Color"),
        "tolerance" => Param::float(0.2).with_label("Tolerance").with_range(0.0, 1.732),
        "softness" => Param::float(0.1).with_label("Softness").with_range(0.0, 1.0),
        "output" => Param::menu("keyed", &["keyed", "matte", "inverted matte"]).with_label("Output"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "tolerance"),
            f(n, c, "softness"),
            menu(n, c, "output"),
            0.0,
        ],
        v4(n, c, "color"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "RGB Key",
            family: Family::Top,
            inputs: &["in"],
            summary: "Key out one colour by distance in RGB.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/rgbkey.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
