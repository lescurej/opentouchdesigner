//! The TOP operator table.
//!
//! Adding an operator is: one `.wgsl` file, one `params` function, one `pack`
//! function, one `TopSpec` entry in `ops/<slug>/mod.rs`. That ratio is deliberate — PLAN.md
//! §6 names operator breadth as the main treadmill risk, so the per-operator
//! cost has to stay near zero.
//!
//! Layout:
//! - `helpers` — parameter evaluation helpers
//! - `blend` — named blend operators (shared composite shader)
//! - `ops/<slug>/mod.rs` — `build.rs` registry

use std::sync::OnceLock;

use otd_core::{Connector, EvalContext, Family, Node, OpDef, OpRegistry};

#[macro_use]
mod helpers;
mod blend;
mod io;
mod video_source;
mod window;
include!(concat!(env!("OUT_DIR"), "/registry_generated.rs"));

pub use cache::CACHE;
pub use chop_to_top::CHOP_TO_TOP;
pub use glsl::{GLSL, shader_source};
pub use in_top::IN;
pub use io::{
    SCREEN_GRAB, VIDEO_STREAM_IN, VIDEO_STREAM_OUT, io_stub_status, is_spout_top, is_syphon_top,
};
pub use out_top::OUT;
pub use spout_in::SPOUT_IN;
pub use spout_out::SPOUT_OUT;
pub use syphon_in::SYPHON_IN;
pub use syphon_out::SYPHON_OUT;
pub use movie_out::{length_limit_frames as movie_out_length_limit_frames, MOVIE_OUT};
pub use select::SELECT;
pub use text::TEXT;
pub use video_device_in::VIDEO_DEVICE_IN;
pub use feedback::FEEDBACK;
pub use movie_in::MOVIE_IN;
pub use null::NULL;
pub use render::RENDER;
pub use video_source::nothing_to_open;
pub use helpers::{
    filter_output_dimensions, generator_size, movie_fit_mode, movie_output_dimensions,
    movie_sampler_nearest,
};

pub const COMMON_WGSL: &str = include_str!("../shaders/common.wgsl");

pub type PackedParams = [[f32; 4]; 4];

pub const PARAM_VECS: usize = 12;

pub type WideParams = [[f32; 4]; PARAM_VECS];

pub const NO_PARAMS: WideParams = [[0.0; 4]; PARAM_VECS];

pub fn widen(packed: PackedParams) -> WideParams {
    let mut out = NO_PARAMS;
    out[..4].copy_from_slice(&packed);
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sizing {
    Params,
    Input0,
    Input0OrParams,
    Referenced,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopCook {
    Shader,
    Feedback,
    MovieIn,
    Render,
    VideoIn,
    MovieOut,
    Ramp,
    Text,
    Analyze,
    ChopToTop,
    Select,
    Cache,
    IoStub,
}

pub struct TopSpec {
    pub def: OpDef,
    pub shader: &'static str,
    pub sizing: Sizing,
    pub two_pass: bool,
    pub dynamic_shader: bool,
    pub pack: fn(&Node, &EvalContext) -> PackedParams,
    pub cook: TopCook,
}

impl TopSpec {
    pub fn pack_params(&self, node: &Node, ctx: &EvalContext) -> WideParams {
        if self.def.type_name == GLSL {
            glsl::pack_wide(node, ctx)
        } else if self.def.type_name == "rampTOP" {
            ramp::pack_wide(node, ctx)
        } else {
            widen((self.pack)(node, ctx))
        }
    }
}

fn specs() -> &'static Vec<TopSpec> {
    static SPECS: OnceLock<Vec<TopSpec>> = OnceLock::new();
    SPECS.get_or_init(folder_specs)
}

pub fn spec(type_name: &str) -> Option<&'static TopSpec> {
    specs().iter().find(|s| s.def.type_name == type_name)
}

pub fn all() -> impl Iterator<Item = &'static TopSpec> {
    specs().iter()
}

pub const WINDOW: &str = "windowCOMP";

pub const WINDOW_COMP: OpDef = OpDef {
    type_name: WINDOW,
    input_families: &[],
    label: "Window",
    family: Family::Comp,
    inputs: &[],
    summary: "Floating or external display for a cooked TOP (TouchDesigner Window COMP).",
    time_dependent: false,
    params: window::params_window_comp,
    connector: Connector::None,
};

pub fn register_all(registry: &mut OpRegistry) {
    for s in specs() {
        registry.register(s.def.clone());
    }
    registry.register(WINDOW_COMP);
}

/// Every operator this build knows about.
pub fn registry() -> OpRegistry {
    let mut r = OpRegistry::new();
    register_all(&mut r);
    r
}

#[cfg(test)]
#[path = "../tests/ops_mod__video_source_tests.rs"]
mod video_source_tests;

#[cfg(test)]
#[path = "../tests/ops_mod__blend_alias_tests.rs"]
mod blend_alias_tests;
