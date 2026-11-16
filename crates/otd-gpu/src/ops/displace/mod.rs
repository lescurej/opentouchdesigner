use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "displaceTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "amount" => Param::float(0.1).with_label("Amount").with_range(-1.0, 1.0),
        "sourcex" => Param::menu("r", &["r", "g", "b", "a", "luminance"]).with_label("Source X"),
        "sourcey" => Param::menu("g", &["r", "g", "b", "a", "luminance"]).with_label("Source Y"),
        "offset" => Param::float(-0.5).with_label("Offset").with_range(-1.0, 1.0),
        "extend" => Param::menu("hold", &["zero", "hold", "repeat", "mirror"]).with_label("Extend"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "amount"),
            menu(n, c, "sourcex"),
            menu(n, c, "sourcey"),
            f(n, c, "offset"),
        ],
        [menu(n, c, "extend"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Displace",
            family: Family::Top,
            inputs: &["source", "displace"],
            summary: "Offsets input 1's lookup by channels of input 2.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/displace.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
