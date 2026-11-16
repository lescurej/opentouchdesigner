use otd_core::indexmap::IndexMap;
use otd_core::{Connector, Family, OpDef, Param};

use super::helpers::*;
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "syphonoutTOP";
pub const SYPHON_OUT: &str = TYPE_NAME;

fn params_syphon_out() -> IndexMap<String, Param> {
    params! {
        "name" => Param::str("OpenTouchDesigner").with_label("Server Name"),
        "active" => Param::bool(true).with_label("Active"),
    }
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Syphon Out",
            family: Family::Top,
            inputs: &["in"],
            summary: "Publishes its input as a Syphon server.",
            time_dependent: true,
            params: params_syphon_out,
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