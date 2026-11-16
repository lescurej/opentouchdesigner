#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "spoutinTOP";
pub const SPOUT_IN: &str = TYPE_NAME;

fn params_spout_in() -> IndexMap<String, Param> {
    params! {
        "name" => Param::str("").with_label("Sender Name"),
        "active" => Param::bool(true).with_label("Active"),
    }
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Spout In",
            family: Family::Top,
            inputs: &[],
            summary: "Receives a Spout texture from another application.",
            time_dependent: true,
            params: params_spout_in,
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