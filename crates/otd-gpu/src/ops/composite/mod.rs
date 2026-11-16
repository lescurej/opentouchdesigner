use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "compositeTOP";

pub fn params() -> IndexMap<String, Param> {
    append_params(
        params! {
            "operation" => Param::menu(
                "over",
                &[
                    "over", "add", "multiply", "screen", "difference", "subtract", "maximum",
                    "minimum", "under", "inside", "outside", "cross",
                ],
            ).with_label("Operation"),
            "opacity" => Param::float(1.0).with_label("Opacity").with_range(0.0, 1.0),
        },
        params_dual_input_sizing(),
    )
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "operation"), f(n, c, "opacity"), 0.0, 0.0],
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
            label: "Composite",
            family: Family::Top,
            inputs: &["base", "over"],
            summary: "Blend two inputs. Input 2 is composited over input 1.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/composite.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}

pub fn blend_alias_spec(
    type_name: &'static str,
    label: &'static str,
    summary: &'static str,
    pack: fn(&Node, &EvalContext) -> PackedParams,
) -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name,
            input_families: &[],
            label,
            family: Family::Top,
            inputs: &["in1", "in2"],
            summary,
            time_dependent: false,
            params: super::blend::params_blend,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/composite.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}