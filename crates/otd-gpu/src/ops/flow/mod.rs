use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "flowTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "amount" => Param::float(6.0).with_label("Amount (px)").with_range(0.0, 128.0),
        "scale" => Param::float(3.0).with_label("Field Scale").with_range(0.1, 32.0),
        "speed" => Param::float(0.3).with_label("Field Speed").with_range(0.0, 8.0),
        "usefield" => Param::bool(false).with_label("Steer From Input 2"),
        "fieldmix" => Param::float(1.0).with_label("Field Mix").with_range(0.0, 1.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            f(n, c, "amount"),
            f(n, c, "scale"),
            f(n, c, "speed"),
            b(n, c, "usefield"),
        ],
        [f(n, c, "fieldmix"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            label: "Flow",
            family: Family::Top,
            inputs: &["in", "field"],
            input_families: &[],
            summary: "Advect the picture along a curl-noise field. Loop it for smoke.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/flow.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}