//! The TOP cook backend — the frame-graph half of PLAN.md §4.
//!
//! `otd-core`'s [`CookEngine`] decides *what* runs; this decides *how*. It
//! implements [`Cooker`], so from core's point of view producing a texture is
//! an opaque side effect.
//!
//! One command encoder is recorded per frame and submitted once. Recording in
//! cook order is what makes the Feedback TOP work: it blits its target's
//! texture at the point in the command stream *before* the target's own pass
//! rewrites it, so it sees last frame's content without any explicit
//! double-buffering.

use std::collections::{HashMap, HashSet};

use otd_core::NodeId;
use slotmap::SecondaryMap;

use crate::context::GpuContext;
use crate::ops::WideParams;
use crate::record::Recorder;
use crate::render3d::Renderer;
use crate::texture::{TexturePool, TopTexture};

mod cook;
mod dispatch;
mod frame;
mod movie_params;
mod pipeline;
mod runtime;
mod setup;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoInKind {
    File,
    Camera,
    Screen,
    Stream,
}

/// Resolution used by an operator with nothing to inherit from.
pub const FALLBACK_SIZE: (u32, u32) = (1280, 720);

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Uniforms {
    res: [f32; 4],
    time: [f32; 4],
    params: WideParams,
}

#[derive(Default)]
pub(super) struct NodeGpu {
    output: Option<TopTexture>,
    /// Depth buffer for a Render TOP, matched to its colour target.
    depth: Option<(u32, u32, wgpu::TextureView)>,
    /// One uniform buffer per pass (separable filters use two).
    uniforms: Vec<wgpu::Buffer>,
    /// Pipeline key of the last shader that compiled, for operators whose
    /// shader comes from a parameter. Kept so a typo mid-edit falls back to
    /// the last working shader instead of going black.
    shader_key: Option<String>,
    shader_error: Option<String>,
    /// A message from something that is not a shader — a missing movie file,
    /// a camera that would not open. Shown on the node body.
    status: Option<String>,
    /// The RGBA8 texture a decoded video frame is uploaded into, and the
    /// decoder revision it holds, so a frame is uploaded once rather than
    /// every time the node cooks.
    upload: Option<Upload>,
}

/// A CPU-written texture: where a decoded frame lands before the GPU sees it.
pub(super) struct Upload {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    revision: u64,
}

pub struct TopEngine {
    ctx: GpuContext,
    bind_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    sampler_linear: wgpu::Sampler,
    sampler_nearest: wgpu::Sampler,
    dummy: wgpu::TextureView,
    dummy_table: wgpu::TextureView,
    const_tables: HashMap<String, wgpu::TextureView>,
    table_override: Option<wgpu::TextureView>,
    pipelines: HashMap<String, wgpu::RenderPipeline>,
    /// Shaders that failed to compile, so a broken source is not recompiled
    /// on every frame.
    failed: HashMap<String, String>,
    /// The fullscreen triangle compiled from GLSL, paired with GLSL fragment
    /// stages. Built on first use so a build with no GLSL shader never pays
    /// for it.
    glsl_vertex: Option<wgpu::ShaderModule>,
    nodes: SecondaryMap<NodeId, NodeGpu>,
    /// One decoder per Movie File In / Video Device In node. Held here
    /// rather than in `NodeGpu` because dropping one kills a thread and a
    /// subprocess, and that must happen exactly when the node goes away.
    videos: SecondaryMap<NodeId, crate::video::Source>,
    /// Audio track of a Movie File In, decoded by AVFoundation on the same
    /// timeline as the picture. Audiomovie reads this; it does not open the
    /// file itself.
    soundtracks: SecondaryMap<NodeId, crate::video::MovieAudioHandle>,
    pool: TexturePool,
    encoder: Option<wgpu::CommandEncoder>,
    /// Scratch targets used by two-pass operators, returned to the pool at
    /// the end of the frame.
    scratch: Vec<TopTexture>,
    /// The 3D pipeline, built on first use so a 2D-only patch never pays for
    /// it.
    renderer: Option<Renderer>,
    /// One encoder subprocess per recording Movie File Out node. Held here
    /// rather than in `NodeGpu` for the reason the video decoders are:
    /// dropping one finalises a file on disk, and that must happen exactly
    /// when the node stops recording, not whenever a texture is recycled.
    recorders: SecondaryMap<NodeId, Recorder>,
    /// Nodes that asked to be recorded this frame, drained in `end_frame`
    /// once the encoder has been submitted and the pixels are real.
    pending_records: Vec<PendingRecord>,
    movie_out_edge: SecondaryMap<NodeId, MovieOutEdge>,
    movie_out_seq: SecondaryMap<NodeId, u32>,
    movie_out_stills: SecondaryMap<NodeId, u64>,
    /// Feedback nodes cooked this frame, with the target they resolved to.
    ///
    /// Their copy happens at the end of the frame rather than at their own
    /// cook — see the Feedback branch in `cook_top` for why order cannot be
    /// relied on.
    pending_feedback: Vec<(NodeId, NodeId)>,
    /// Target picture captured at end of frame *n*; applied to the Feedback TOP
    /// at the start of frame *n+1* so reads after `end_frame` still lag one frame.
    feedback_snapshot: SecondaryMap<NodeId, TopTexture>,
    /// Last completed TOP→CHOP readbacks (one frame late — see operator docs).
    chop_pixel_cache: HashMap<NodeId, (u32, u32, Vec<f32>)>,
    chop_pixel_wanted: HashSet<NodeId>,
    chop_pixel_pending: HashMap<NodeId, crate::PendingPixelReadback>,
    /// Parsed font faces, shared by every Text TOP.
    fonts: crate::text::FontCache,
    /// What each Text TOP last rasterised, so retyping one caption does not
    /// re-lay-out every other one — and so a static caption costs nothing at
    /// all after its first frame.
    text_keys: SecondaryMap<NodeId, String>,
    movie_ui: SecondaryMap<NodeId, MovieUiEdge>,
    pub passes_this_frame: u32,
    sop_previews: SecondaryMap<NodeId, SopPreviewGpu>,
    defer_submit: bool,
    deferred_gpu: Vec<DeferredGpuFrame>,
}

pub(super) struct DeferredGpuFrame {
    commands: wgpu::CommandBuffer,
    records: Vec<PendingRecord>,
    scratch: Vec<TopTexture>,
}

/// Offscreen target for a SOP network viewer.
pub(super) struct SopPreviewGpu {
    output: TopTexture,
    depth: wgpu::TextureView,
    cache_key: u64,
}

/// Square resolution for SOP viewer thumbnails on the network.
pub const SOP_PREVIEW_SIZE: u32 = 384;

#[derive(Default)]
pub(super) struct MovieUiEdge {
    reload: bool,
    cue_pulse: bool,
    cue: bool,
    play: bool,
    last_playmode: String,
}

#[derive(Default)]
pub(super) struct MovieOutEdge {
    record: bool,
    addframe: bool,
}

pub(super) enum PendingRecord {
    Stream(NodeId),
    Still {
        id: NodeId,
        path: String,
        image_type: String,
    },
}

impl TopEngine {
    fn movie_edge(&mut self, id: NodeId) -> &mut MovieUiEdge {
        if !self.movie_ui.contains_key(id) {
            self.movie_ui.insert(id, MovieUiEdge::default());
        }
        self.movie_ui.get_mut(id).unwrap()
    }
}

#[cfg(test)]
#[path = "../tests/engine_mod__const_table_pipeline.rs"]
mod const_table_pipeline;
