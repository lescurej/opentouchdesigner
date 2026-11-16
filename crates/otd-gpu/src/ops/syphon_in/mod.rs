use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "syphoninTOP";
pub const SYPHON_IN: &str = TYPE_NAME;

fn params_syphon_in() -> IndexMap<String, Param> {
    params! {
        "name" => Param::str("").with_label("Server Name"),
        "active" => Param::bool(true).with_label("Active"),
    }
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Syphon In",
            family: Family::Top,
            inputs: &[],
            summary: "Receives a Syphon texture from another application.",
            time_dependent: true,
            params: params_syphon_in,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::IoStub,
    }
}