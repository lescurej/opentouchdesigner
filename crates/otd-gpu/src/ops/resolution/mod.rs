use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "resolutionTOP";

fn params() -> IndexMap<String, Param> {
    with_res(params! {
        "filter" => Param::menu("linear", &["linear", "nearest"]).with_label("Filter"),
    })
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Resolution",
            family: Family::Top,
            inputs: &["in"],
            summary: "Resamples its input to an explicit resolution.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::Shader,
    }
}