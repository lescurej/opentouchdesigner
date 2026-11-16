use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "matteTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "source" => Param::menu("alpha", &["alpha", "luminance", "red"]).with_label("Matte From"),
        "invert" => Param::bool(false).with_label("Invert"),
        "premultiply" => Param::bool(false).with_label("Premultiply"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "source"),
            b(n, c, "invert"),
            b(n, c, "premultiply"),
            0.0,
        ],
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
            label: "Matte",
            family: Family::Top,
            inputs: &["source", "matte"],
            summary: "Colour from input 1, alpha from input 2.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/matte.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
