use otd_core::{Connector, Family, OpDef};

use super::helpers::{pack_none, params_none};
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = "nullTOP";
pub const NULL: &str = TYPE_NAME;

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Null",
            family: Family::Top,
            inputs: &["in"],
            summary: "Pass-through. A stable name to reference and to view.",
            time_dependent: false,
            params: params_none,
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