use std::sync::{Arc, Mutex};
use std::time::Instant;

use otd_core::{CookContext, Graph, NodeId, OpRegistry};
use otd_engine::{FrameConfig, FrameResult, PlaybackState, ProjectRuntime, TickHint};
use otd_gpu::GpuContext;

pub struct UiEngine {
    pub runtime: Arc<Mutex<ProjectRuntime>>,
    last_tick: Instant,
}

impl UiEngine {
    pub fn new(graph: Graph, registry: OpRegistry, time: CookContext, gpu: GpuContext) -> Self {
        UiEngine {
            runtime: Arc::new(Mutex::new(ProjectRuntime::new(graph, registry, time, gpu))),
            last_tick: Instant::now(),
        }
    }

    pub fn runtime(&self) -> &Arc<Mutex<ProjectRuntime>> {
        &self.runtime
    }

    pub fn cook_frame(
        &mut self,
        graph: &mut Graph,
        time: &mut CookContext,
        hint: TickHint,
    ) -> (FrameResult, PlaybackState) {
        let now = Instant::now();
        let wall_dt = (now - self.last_tick).as_secs_f64().max(1e-6);
        self.last_tick = now;
        let dt = wall_dt;
        let mut rt = self.runtime.lock().expect("engine runtime lock");
        rt.graph.clone_from(graph);
        rt.time.clone_from(time);
        rt.graph_shared.refresh_generation();
        let mut playback = hint.playback;
        let result = rt.tick_frame(
            FrameConfig {
                dt,
                roots: hint.roots,
                audio_latency_ms: hint.audio_latency_ms,
                input: hint.input,
                display_blits: hint.display_blits,
                defer_gpu_submit: hint.playback.playing,
                max_time_slice_ms: hint.max_time_slice_ms,
                advance_timeline: hint.advance_timeline,
            },
            &mut playback,
        );
        graph.clone_from(&rt.graph);
        time.clone_from(&rt.time);
        (result, playback)
    }

    fn with_rt<R>(&self, f: impl FnOnce(&ProjectRuntime) -> R) -> R {
        let rt = self.runtime.lock().expect("engine runtime lock");
        f(&rt)
    }

    pub fn top_output_meta(&self, graph: &Graph, id: NodeId) -> Option<(u32, u32, u64)> {
        self.with_rt(|rt| {
            let tex = rt.engines.top.output(graph, id)?;
            Some((tex.key.width, tex.key.height, tex.generation))
        })
    }

    pub fn reset_cook_state(&self) {
        if let Ok(mut rt) = self.runtime.lock() {
            rt.engines.reset();
            rt.cook.reset();
        }
    }

    pub fn forget_node(&self, id: NodeId) {
        if let Ok(mut rt) = self.runtime.lock() {
            rt.engines.forget(id);
            rt.cook.forget(id);
        }
    }

    pub fn python_error(&self) -> Option<String> {
        self.with_rt(|rt| rt.engines.python_error())
    }

    pub fn cook_frame_cost_ms(&self, id: NodeId) -> f64 {
        self.with_rt(|rt| rt.cook.frame_cost_ms(id))
    }

    pub fn is_time_dependent(&self, id: NodeId) -> bool {
        self.with_rt(|rt| rt.cook.is_time_dependent(id))
    }

    pub fn top_gpu_stats(&self) -> (u32, u64, u64, u64) {
        self.with_rt(|rt| {
            (
                rt.engines.top.passes_this_frame,
                rt.engines.top.resident_bytes(),
                rt.engines.top.pooled_bytes(),
                rt.engines.top.textures_created(),
            )
        })
    }

    pub fn read_top_rgba8(&self, graph: &Graph, id: NodeId) -> Result<(u32, u32, Vec<u8>), String> {
        let rt = self
            .runtime
            .lock()
            .map_err(|e| format!("runtime lock: {e}"))?;
        let tex = rt
            .engines
            .top
            .output(graph, id)
            .ok_or_else(|| format!("{} produced no texture", graph.path(id)))?;
        otd_gpu::read_pixels_rgba8(&rt.gpu, tex).map_err(|e| e.to_string())
    }

    pub fn poll_code_lint(
        &self,
        cache: &mut crate::code_lint::CodeLintCache,
        id: egui::Id,
        now: f64,
        graph: &Graph,
        me_path: &str,
        filename: &str,
    ) {
        let rt = self.runtime.lock().expect("engine runtime lock");
        cache.poll(id, now, &rt.engines, graph, me_path, filename);
    }
}

pub fn playback_from_app(
    playing: bool,
    looping: bool,
    scrubbing: bool,
    loop_range: (f64, f64),
) -> PlaybackState {
    PlaybackState {
        playing,
        looping,
        scrubbing,
        loop_range,
    }
}
