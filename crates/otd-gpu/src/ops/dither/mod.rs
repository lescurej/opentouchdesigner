use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "ditherTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "levels" => Param::int(4).with_label("Levels").with_range(2.0, 64.0),
        "pattern" => Param::menu("bayer4", &["bayer4", "bayer8", "noise", "none"])
            .with_label("Pattern"),
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 1.0),
        "scale" => Param::float(1.0).with_label("Pixel Size").with_range(1.0, 32.0),
        "monochrome" => Param::bool(false).with_label("Monochrome"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "levels"),
            menu(n, c, "pattern"),
            f(n, c, "strength"),
            b(n, c, "monochrome"),
        ],
        [f(n, c, "scale"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Dither",
            family: Family::Top,
            inputs: &["in"],
            summary: "Quantise to few levels, with an ordered or noise dither.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/dither.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
