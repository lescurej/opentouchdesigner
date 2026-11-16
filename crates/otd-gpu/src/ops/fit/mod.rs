use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "fitTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "mode" => Param::menu("fit", &["fill", "fit", "stretch", "native"]).with_label("Fit"),
        "background" => Param::rgba([0.0, 0.0, 0.0, 0.0]).with_label("Background"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "mode"), 0.0, 0.0, 0.0],
        v4(n, c, "background"),
        [0.0; 4],
        [0.0; 4],
    ]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Fit",
            family: Family::Top,
            inputs: &["in"],
            summary: "Resample into a new resolution: fill, fit, stretch or native.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/fit.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
