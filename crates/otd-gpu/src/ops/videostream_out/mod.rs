use otd_core::{Connector, EvalContext, Family, Node, OpDef};

use super::helpers::pack_none;
use super::io::{self, VIDEO_STREAM_OUT};
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = VIDEO_STREAM_OUT;

fn params() -> otd_core::indexmap::IndexMap<String, otd_core::Param> {
    io::params_videostream_out()
}

fn pack(_n: &Node, _c: &EvalContext) -> super::PackedParams {
    pack_none(_n, _c)
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Video Stream Out",
            family: Family::Top,
            inputs: &["in"],
            summary: "Sends its input to a network stream URL.",
            time_dependent: false,
            params,
            connector: Connector::None,
        },
        shader: include_str!("../../shaders/null.wgsl"),
        sizing: Sizing::Input0,
        two_pass: false,
        dynamic_shader: false,
        pack,
        cook: TopCook::Shader,
    }
}