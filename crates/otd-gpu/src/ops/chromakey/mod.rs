use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "chromakeyTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "key" => Param::rgba([0.0, 1.0, 0.0, 1.0]).with_label("Key Colour"),
        "tolerance" => Param::float(0.15).with_label("Tolerance").with_range(0.0, 1.0),
        "softness" => Param::float(0.1).with_label("Softness").with_range(0.0, 1.0),
        "despill" => Param::float(1.0).with_label("Despill").with_range(0.0, 1.0),
        "replace" => Param::bool(false).with_label("Replace Rather Than Cut"),
        "replacement" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Replacement"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    let key = v4(n, c, "key");
    [
        [key[0], key[1], key[2], f(n, c, "tolerance")],
        [
            f(n, c, "softness"),
            f(n, c, "despill"),
            b(n, c, "replace"),
            0.0,
        ],
        v4(n, c, "replacement"),
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Chroma Key",
            family: Family::Top,
            inputs: &["in"],
            summary: "Key out one colour, matching on hue rather than brightness.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/chromakey.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
