use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "functionTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "function" => Param::menu(
            "none",
            &[
                "none", "log", "exp", "sqrt", "square", "reciprocal", "invert",
                "sin", "cos", "atan", "abs", "sign",
            ],
        ).with_label("Function"),
        "premultiply" => Param::float(1.0).with_label("Pre Multiply").with_range(-4.0, 4.0),
        "preadd" => Param::float(0.0).with_label("Pre Add").with_range(-4.0, 4.0),
        "postmultiply" => Param::float(1.0).with_label("Post Multiply").with_range(-4.0, 4.0),
        "postadd" => Param::float(0.0).with_label("Post Add").with_range(-4.0, 4.0),
        "alpha" => Param::bool(false).with_label("Include Alpha"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [
            menu(n, c, "function"),
            f(n, c, "premultiply"),
            f(n, c, "preadd"),
            f(n, c, "postmultiply"),
        ],
        [f(n, c, "postadd"), b(n, c, "alpha"), 0.0, 0.0],
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Function",
            family: Family::Top,
            inputs: &["in"],
            summary: "One unary function per pixel, with a scale and offset either side.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/function.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
