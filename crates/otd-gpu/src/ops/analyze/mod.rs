use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "analyzeTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "function" => Param::menu("average", &["average", "minimum", "maximum"])
            .with_label("Function")
            .with_page("Analyze"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [[0.0, 0.0, menu(n, c, "function"), 0.0], [0.0; 4], [0.0; 4], [0.0; 4]]
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Analyze",
            family: Family::Top,
            inputs: &["in"],
            summary: "Reduces the input to one pixel: average, minimum or maximum.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Analyze,
    }
}
