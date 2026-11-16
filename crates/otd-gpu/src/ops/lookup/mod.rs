use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "lookupTOP";

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "source" => Param::menu("luminance", &["luminance", "red", "green", "blue", "alpha"])
            .with_label("Index By"),
    })
}

fn pack(n: &Node, c: &EvalContext) -> PackedParams {
    [
        [menu(n, c, "source"), 0.0, 0.0, 0.0],
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
            label: "Lookup",
            family: Family::Top,
            inputs: &["index", "table"],
            summary: "Input 1's brightness reads a colour out of input 2.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/lookup.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}
