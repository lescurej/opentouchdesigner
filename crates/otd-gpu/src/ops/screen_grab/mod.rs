use otd_core::{Connector, EvalContext, Family, Node, OpDef};

use super::helpers::pack_none;
use super::io::{self, SCREEN_GRAB};
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = SCREEN_GRAB;

fn params() -> otd_core::indexmap::IndexMap<String, otd_core::Param> {
    io::params_screen_grab()
}

fn pack(_n: &Node, _c: &EvalContext) -> super::PackedParams {
    pack_none(_n, _c)
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Screen Grab",
            family: Family::Top,
            inputs: &[],
            summary: "Captures the desktop through ffmpeg.",
            time_dependent: true,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Params,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::VideoIn,
    }
}