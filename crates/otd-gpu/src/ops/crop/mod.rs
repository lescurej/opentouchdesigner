use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "cropTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "left" => Param::float(0.0).with_label("Left").with_range(0.0, 1.0),
        "right" => Param::float(1.0).with_label("Right").with_range(0.0, 1.0),
        "bottom" => Param::float(0.0).with_label("Bottom").with_range(0.0, 1.0),
        "top" => Param::float(1.0).with_label("Top").with_range(0.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "left"),
            f(n, c, "right"),
            f(n, c, "bottom"),
            f(n, c, "top"),
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
            label: "Crop",
            family: Family::Top,
            inputs: &["in"],
            summary: "Keep a rectangle of the source, at an explicit output size.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/crop.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
