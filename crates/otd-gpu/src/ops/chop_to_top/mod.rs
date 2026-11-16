use otd_core::indexmap::IndexMap;
use otd_core::{Connector, EvalContext, Family, Node, OpDef, Param};

use super::helpers::*;
use super::{PackedParams, Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "choptotopTOP";
pub const CHOP_TO_TOP: &str = TYPE_NAME;

fn params() -> IndexMap<String, Param> {
    merge_common(params! {
        "chop" => Param::str("").with_label("CHOP").as_path_ref(),
        "layout" => Param::menu("mono", &["mono", "rgba"])
            .with_label("Channel Layout"),
    })
}


fn pack(_n: &Node, _c: &EvalContext) -> PackedParams {
    [[0.0; 4]; 4]
}


pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[Family::Chop],
            label: "CHOP to TOP",
            family: Family::Top,
            inputs: &["in"],
            summary: "Channels as pixels: a row per channel, a column per sample.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Referenced,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::ChopToTop,
    }
}
