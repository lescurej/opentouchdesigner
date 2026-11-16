use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "slopeTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "mode" => Param::menu("xy", &["xy", "magnitude", "direction"]).with_label("Output"),
        "strength" => Param::float(1.0).with_label("Strength").with_range(0.0, 32.0),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "mode"), f(n, c, "strength"), 0.0, 0.0],
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
            label: "Slope",
            family: Family::Top,
            inputs: &["in"],
            summary: "The image's own gradient, as a vector, a magnitude or an angle.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/slope.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
