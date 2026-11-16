use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "normalmapTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "source" => Param::menu("luminance", &["luminance", "alpha", "red"]).with_label("Height From"),
        "strength" => Param::float(4.0).with_label("Strength").with_range(0.0, 64.0),
        "flipgreen" => Param::bool(false).with_label("Flip Green"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "source"),
            f(n, c, "strength"),
            b(n, c, "flipgreen"),
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
            label: "Normal Map",
            family: Family::Top,
            inputs: &["in"],
            summary: "A height field becomes a tangent-space normal map.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/normalmap.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
