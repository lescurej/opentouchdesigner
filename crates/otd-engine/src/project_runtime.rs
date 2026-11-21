//! Headless and editor share one frame sequence — advance, sync, cook, callbacks.

use std::path::Path;
use std::time::Instant;

use otd_chop::InputState;
use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Project, SharedGraph, edit};
use otd_gpu::GpuContext;

use crate::{Engines, replicator};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FrameTiming {
    pub cook_ms: f64,
    /// CPU time inside the cook graph for this tick.
    pub wall_ms: f64,
    /// Wall clock since the previous tick (what the timeline and FPS use).
    pub interval_ms: f64,
    pub cooked: u32,
    pub cached: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PlaybackState {
    pub playing: bool,
    pub looping: bool,
    pub scrubbing: bool,
    pub loop_range: (f64, f64),
}

#[derive(Clone)]
pub struct EguiDisplayBlit {
    pub top_id: NodeId,
    pub dst: wgpu::TextureView,
}

#[derive(Clone)]
pub struct FrameConfig {
    pub dt: f64,
    pub roots: Vec<NodeId>,
    pub audio_latency_ms: u32,
    pub input: InputState,
    pub display_blits: Vec<EguiDisplayBlit>,
    pub defer_gpu_submit: bool,
    pub max_time_slice_ms: f64,
    pub advance_timeline: bool,
}

impl Default for FrameConfig {
    fn default() -> Self {
        FrameConfig {
            dt: 0.0,
            roots: Vec::new(),
            audio_latency_ms: 0,
            input: InputState::default(),
            display_blits: Vec::new(),
            defer_gpu_submit: false,
            max_time_slice_ms: otd_core::project::DEFAULT_MAX_TIME_SLICE_MS,
            advance_timeline: true,
        }
    }
}

pub fn clip_frame_dt(dt: f64, max_time_slice_ms: f64) -> f64 {
    let cap = max_time_slice_ms.max(1.0) / 1000.0;
    dt.max(1e-6).min(cap)
}

impl std::fmt::Debug for FrameConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FrameConfig")
            .field("dt", &self.dt)
            .field("roots", &self.roots)
            .field("audio_latency_ms", &self.audio_latency_ms)
            .field("input", &self.input)
            .field("display_blits", &self.display_blits.len())
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct GraphSyncStats {
    pub replicated: usize,
    pub clones_synced: usize,
}

#[derive(Debug, Default)]
pub struct CallbackFrameStats {
    pub param_edits_applied: usize,
    pub param_edit_problems: Vec<String>,
    pub recooks_applied: usize,
    pub script_udp_sent: usize,
    pub script_log: Vec<String>,
}

#[derive(Debug)]
pub struct FrameResult {
    pub timing: FrameTiming,
    pub cook_error: Option<String>,
    pub graph_sync: GraphSyncStats,
    pub callbacks: CallbackFrameStats,
}

pub struct ProjectRuntime {
    pub graph: Graph,
    pub graph_shared: SharedGraph,
    pub registry: OpRegistry,
    pub engines: Engines,
    pub cook: CookEngine,
    pub time: CookContext,
    pub gpu: GpuContext,
}

impl ProjectRuntime {
    pub fn new(graph: Graph, registry: OpRegistry, time: CookContext, gpu: GpuContext) -> Self {
        let graph_shared = SharedGraph::new(graph.clone());
        Self {
            engines: Engines::new(gpu.clone()),
            cook: CookEngine::new(),
            graph,
            graph_shared,
            registry,
            time,
            gpu,
        }
    }

    pub fn open(path: &Path) -> Result<Self, String> {
        let registry = crate::registry();
        let project = Project::load(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let fps = project.fps;
        let sample_rate = otd_core::canonical_project_sample_rate(project.sample_rate);
        let graph = project
            .to_graph_relative_to(&registry, path.parent())
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let gpu = GpuContext::headless().map_err(|e| format!("no GPU: {e}"))?;
        let time = CookContext {
            fps,
            sample_rate,
            dt: 1.0 / fps.max(1e-6),
            ..Default::default()
        };
        Ok(Self::new(graph, registry, time, gpu))
    }

    pub fn advance_playback(
        time: &mut CookContext,
        playback: &mut PlaybackState,
        dt: f64,
        advance_timeline: bool,
    ) {
        let dt = dt.max(1e-6);
        if advance_timeline && playback.playing && !playback.scrubbing {
            time.advance(dt);
            let (start, end) = playback.loop_range;
            if playback.looping && end > start && time.time >= end {
                let length = end - start;
                time.time = start + (time.time - start).rem_euclid(length);
            }
        } else {
            time.dt = dt;
        }
        playback.scrubbing = false;
    }

    /// Cook one frame: sync graph, run roots, apply callback side effects.
    pub fn tick_frame(&mut self, cfg: FrameConfig, playback: &mut PlaybackState) -> FrameResult {
        self.graph_shared.refresh_generation();
        tick_frame_state(
            &mut self.graph,
            &self.registry,
            &mut self.engines,
            &mut self.cook,
            &mut self.time,
            cfg,
            playback,
        )
    }
}

/// Same sequence as [`ProjectRuntime::tick_frame`] on detached state (editor owns GPU).
pub fn tick_frame_state(
    graph: &mut Graph,
    registry: &OpRegistry,
    engines: &mut Engines,
    cook: &mut CookEngine,
    time: &mut CookContext,
    cfg: FrameConfig,
    playback: &mut PlaybackState,
) -> FrameResult {
    let started = Instant::now();
    let dt = clip_frame_dt(cfg.dt, cfg.max_time_slice_ms);
    ProjectRuntime::advance_playback(time, playback, dt, cfg.advance_timeline);
    engines.chop.io.perform = otd_chop::PerformFrameStats::from_cook(
        time,
        engines.last_frame_timing.cook_ms,
        engines.last_frame_timing.interval_ms.max(1e-3),
    );

    let replicated = replicator::sync(graph, &engines.dats);
    let clones_synced = graph.sync_clones(registry);
    let generation = graph.audio_topology_generation();
    engines.audio_partition = crate::audio_partition::partition_audio_domain(graph, generation);
    engines.audio_rt.tick(
        graph,
        &engines.audio_partition,
        time,
        &mut engines.channels,
        &mut engines.chop.io,
        time.sample_rate,
    );
    engines
        .chop
        .io
        .set_audio_rt_armed(engines.audio_rt.is_armed());

    engines.set_audio_latency_ms(cfg.audio_latency_ms);
    engines.set_input_state(cfg.input);
    engines.set_defer_playback_submit(cfg.defer_gpu_submit);
    engines.begin_frame();
    let cook_result = engines.run_cook_frame(cook, graph, &cfg.roots, &time.clone());
    if !cfg.display_blits.is_empty() {
        engines.run_egui_display_blits(graph, time, &cfg.display_blits);
    }
    engines.end_frame(graph);

    let cook_error = if cook_result.is_err() {
        cook.clear_visiting();
        cook_result.err().map(|e| e.to_string())
    } else {
        None
    };

    let edits = engines.take_edits();
    let (param_edits_applied, param_edit_problems) = if edits.is_empty() {
        (0, Vec::new())
    } else {
        edit::apply(graph, &edits)
    };

    let recooks = engines.take_recook_requests();
    let recooks_applied = if recooks.is_empty() {
        0
    } else {
        edit::apply_recooks(graph, &recooks)
    };

    let script_udp_sent = engines.apply_script_udp_sends(graph);
    let script_log = engines.take_script_log();

    let timing = FrameTiming {
        cook_ms: cook.stats.total_cook_us as f64 / 1000.0,
        wall_ms: started.elapsed().as_secs_f64() * 1000.0,
        interval_ms: dt * 1000.0,
        cooked: cook.stats.cooked,
        cached: cook.stats.cached,
    };
    engines.last_frame_timing = timing;

    FrameResult {
        timing,
        cook_error,
        graph_sync: GraphSyncStats {
            replicated,
            clones_synced,
        },
        callbacks: CallbackFrameStats {
            param_edits_applied,
            param_edit_problems,
            recooks_applied,
            script_udp_sent,
            script_log,
        },
    }
}

impl ProjectRuntime {
    pub fn read_top_rgba8(&self, id: NodeId) -> Result<(u32, u32, Vec<u8>), String> {
        let tex = self
            .engines
            .top
            .output(&self.graph, id)
            .ok_or_else(|| format!("{} produced no texture", self.graph.path(id)))?;
        otd_gpu::read_pixels_rgba8(&self.gpu, tex)
    }
}

/// Median and tail of frame timings — mean hides hitches.
pub fn summarize(mut values: Vec<f64>) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = values[values.len() / 2];
    let p95 = values[(values.len() * 95 / 100).min(values.len() - 1)];
    let max = *values.last().unwrap();
    (median, p95, max)
}

#[cfg(test)]
#[path = "tests/project_runtime__clip_tests.rs"]
mod clip_tests;

#[cfg(test)]
#[path = "tests/project_runtime__tests.rs"]
mod tests;
