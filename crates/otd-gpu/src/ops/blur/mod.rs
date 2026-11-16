use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "blurTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "size" => Param::float(8.0).with_label("Size (px)").with_range(0.0, 512.0),
        "preshrink" => Param::int(1).with_label("Pre-Shrink").with_range(1.0, 16.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [f(n, c, "size"), 0.0, f(n, c, "preshrink"), 0.0],
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
            label: "Blur",
            family: Family::Top,
            inputs: &["in"],
            summary: "Two-pass box blur with an optional pre-shrink.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/blur.wgsl"),
        sizing: Sizing::Input0,
        two_pass: true,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}