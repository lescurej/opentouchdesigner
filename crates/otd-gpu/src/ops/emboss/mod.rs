use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "embossTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "direction" => Param::float(45.0).with_label("Direction").with_range(-180.0, 180.0),
        "width" => Param::float(1.0).with_label("Width (px)").with_range(0.0, 32.0),
        "strength" => Param::float(4.0).with_label("Strength").with_range(0.0, 32.0),
        "mix" => Param::float(0.0).with_label("Mix With Source").with_range(0.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "direction").to_radians(),
            f(n, c, "width"),
            f(n, c, "strength"),
            f(n, c, "mix"),
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
            label: "Emboss",
            family: Family::Top,
            inputs: &["in"],
            summary: "A directional difference, sat at mid grey.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/emboss.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
