use otd_core::{Connector, Family, OpDef};

use super::helpers::{pack_none, params_none};
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "inTOP";
pub const IN: &str = TYPE_NAME;

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "In",
            family: Family::Top,
            inputs: &[],
            summary: "A texture input on this component's node.",
            time_dependent: false,
            params: params_none,
            connector: Connector::In,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Referenced,
        two_pass: false,
        dynamic_shader: false,
        pack: pack_none,
        cook: TopCook::Shader,
    }
}