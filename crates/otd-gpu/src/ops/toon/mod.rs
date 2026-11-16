use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "toonTOP";

fn params() -> IndexMap<String, Param> {
    params! {
        "bands" => Param::int(4).with_label("Bands").with_range(2.0, 32.0),
        "edge" => Param::float(2.0).with_label("Ink Strength").with_range(0.0, 16.0),
        "edgewidth" => Param::float(1.0).with_label("Ink Width (px)").with_range(0.0, 8.0),
        "saturation" => Param::float(1.3).with_label("Saturation").with_range(0.0, 4.0),
        "inkcolor" => Param::rgba([0.0, 0.0, 0.0, 1.0]).with_label("Ink Color"),
    }
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "bands"),
            f(n, c, "edge"),
            f(n, c, "edgewidth"),
            f(n, c, "saturation"),
        ],
        v4(n, c, "inkcolor"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            label: "Toon",
            family: Family::Top,
            inputs: &["in"],
            input_families: &[],
            summary: "Cel shading: flatten the luminance into bands and ink the edges.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/toon.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}