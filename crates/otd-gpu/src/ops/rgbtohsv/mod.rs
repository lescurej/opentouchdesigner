use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::{merge_common, pack_none, params_none};
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "rgbtohsvTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params_none())
}

fn pack(_n: &Node, _c: &EvalContext) -> PackedParams {
    pack_none(_n, _c)
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "RGB to HSV",
            family: Family::Top,
            inputs: &["in"],
            summary: "Hue, saturation and value land in the red, green and blue channels.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/rgbtohsv.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
