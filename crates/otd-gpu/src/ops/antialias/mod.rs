use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "antialiasTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 2.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [f(n, c, "strength"), 0.0, 0.0, 0.0],
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
            label: "Anti Alias",
            family: Family::Top,
            inputs: &["in"],
            summary: "Softens luminance edges in place — FXAA as a post pass.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/antialias.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
