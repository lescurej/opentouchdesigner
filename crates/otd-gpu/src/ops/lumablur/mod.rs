use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "lumablurTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "white" => Param::float(16.0).with_label("White Radius (px)").with_range(0.0, 128.0),
        "black" => Param::float(0.0).with_label("Black Radius (px)").with_range(0.0, 128.0),
        "source" => Param::menu("luminance", &["luminance", "alpha", "red"]).with_label("Radius From"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "white"),
            f(n, c, "black"),
            menu(n, c, "source"),
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
            label: "Luma Blur",
            family: Family::Top,
            inputs: &["source", "blur"],
            summary: "Blur radius driven by input 2's brightness.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/lumablur.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
