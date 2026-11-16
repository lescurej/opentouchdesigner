use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "limitTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "mode" => Param::menu("clamp", &["clamp", "quantize", "both"]).with_label("Mode"),
        "min" => Param::float(0.0).with_label("Minimum").with_range(-2.0, 2.0),
        "max" => Param::float(1.0).with_label("Maximum").with_range(-2.0, 2.0),
        "step" => Param::float(0.25).with_label("Quantize Step").with_range(0.001, 1.0),
        "alpha" => Param::bool(false).with_label("Include Alpha"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "mode"),
            f(n, c, "min"),
            f(n, c, "max"),
            f(n, c, "step"),
        ],
        [b(n, c, "alpha"), 0.0, 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Limit",
            family: Family::Top,
            inputs: &["in"],
            summary: "Clamp to a range, quantise to a step, or both.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/limit.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
