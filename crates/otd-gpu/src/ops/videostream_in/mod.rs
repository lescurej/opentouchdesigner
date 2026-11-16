use otd_core::{Connector, EvalContext, Family, Node, OpDef};

use super::helpers::pack_none;
use super::io::{self, VIDEO_STREAM_IN};
use super::{Sizing, TopCook, TopSpec};

pub const TYPE_NAME: &str = VIDEO_STREAM_IN;

fn params() -> otd_core::indexmap::IndexMap<String, otd_core::Param> {
    io::params_videostream_in()
}

fn pack(_n: &Node, _c: &EvalContext) -> super::PackedParams {
    pack_none(_n, _c)
}

pub fn spec() -> TopSpec {
    TopSpec {
        def: OpDef {
            type_name: TYPE_NAME,
            input_families: &[],
            label: "Video Stream In",
            family: Family::Top,
            inputs: &[],
            summary: "Reads a network stream (RTSP, HTTP, HLS) through ffmpeg.",
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