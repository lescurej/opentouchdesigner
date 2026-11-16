#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "spoutoutTOP";
pub const SPOUT_OUT: &str = TYPE_NAME;

fn params_spout_out() -> IndexMap<String, Param> {
    params! {
        "name" => Param::str("OpenTouchDesigner").with_label("Sender Name"),
        "active" => Param::bool(true).with_label("Active"),
    }
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Spout Out",
            family: Family::Top,
            inputs: &["in"],
            summary: "Publishes its input as a Spout sender.",
            time_dependent: true,
            params: params_spout_out,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::Shader,
    }
}