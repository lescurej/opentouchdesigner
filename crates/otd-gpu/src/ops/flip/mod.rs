use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "flipTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "flipx" => Param::bool(true).with_label("Flip Horizontally"),
        "flipy" => Param::bool(false).with_label("Flip Vertically"),
        "transpose" => Param::bool(false).with_label("Transpose"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            b(n, c, "flipx"),
            b(n, c, "flipy"),
            b(n, c, "transpose"),
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
            label: "Flip",
            family: Family::Top,
            inputs: &["in"],
            summary: "Mirror the image about an axis, or transpose it.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/flip.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
