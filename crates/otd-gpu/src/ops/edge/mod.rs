use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "edgeTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 8.0),
        "width" => Param::float(1.0).with_label("Width (px)").with_range(0.0, 16.0),
        "direction" => Param::menu("both", &["both", "horizontal", "vertical"])
            .with_label("Direction"),
        "keepcolor" => Param::float(0.0).with_label("Keep Colour").with_range(0.0, 1.0),
        "color" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Edge Colour"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "strength"),
            f(n, c, "width"),
            menu(n, c, "direction"),
            f(n, c, "keepcolor"),
        ],
        v4(n, c, "color"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Edge",
            family: Family::Top,
            inputs: &["in"],
            summary: "Sobel edge detection.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/edge.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
