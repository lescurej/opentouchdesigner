//! Dedicated cook thread — wall-clock ticks, shared [`ProjectRuntime`] for readback.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use otd_chop::InputState;
use otd_core::{CookContext, Graph, NodeId, OpRegistry};
use otd_gpu::GpuContext;

use crate::project_runtime::{FrameConfig, FrameResult, PlaybackState, ProjectRuntime};

#[derive(Clone)]
pub struct TickHint {
    pub graph: Graph,
    pub time: CookContext,
    pub roots: Vec<NodeId>,
    pub playback: PlaybackState,
    pub input: InputState,
    pub audio_latency_ms: u32,
    pub fps: f64,
    pub display_blits: Vec<crate::project_runtime::EguiDisplayBlit>,
    pub defer_gpu_submit: bool,
    pub max_time_slice_ms: f64,
    pub advance_timeline: bool,
}

impl Default for TickHint {
    fn default() -> Self {
        TickHint {
            graph: Graph::new(),
            time: CookContext::default(),
            roots: Vec::new(),
            playback: PlaybackState::default(),
            input: InputState::default(),
            audio_latency_ms: 0,
            fps: otd_core::project::DEFAULT_PROJECT_FPS,
            display_blits: Vec::new(),
            defer_gpu_submit: false,
            max_time_slice_ms: otd_core::project::DEFAULT_MAX_TIME_SLICE_MS,
            advance_timeline: true,
        }
    }
}

pub struct FrameEvent {
    pub time: CookContext,
    pub playback: PlaybackState,
    pub result: FrameResult,
}

struct LoopControl {
    hint: Mutex<TickHint>,
    stop: AtomicBool,
}

/// Background cook loop. The UI updates the runtime graph, sets [`TickHint`], polls [`FrameEvent`]s.
pub struct EngineClient {
    runtime: Arc<Mutex<ProjectRuntime>>,
    control: Arc<LoopControl>,
    event_rx: std::sync::mpsc::Receiver<FrameEvent>,
    join: Option<JoinHandle<()>>,
}

impl EngineClient {
    pub fn spawn(
        graph: Graph,
        registry: OpRegistry,
        time: CookContext,
        gpu: GpuContext,
    ) -> Result<Self, String> {
        let runtime = Arc::new(Mutex::new(ProjectRuntime::new(
            graph,
            registry,
            time,
            gpu,
        )));
        let control = Arc::new(LoopControl {
            hint: Mutex::new(TickHint {
                fps: time.fps,
                ..Default::default()
            }),
            stop: AtomicBool::new(false),
        });
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let worker_rt = runtime.clone();
        let worker_ctl = control.clone();
        let join = thread::Builder::new()
            .name("otd-engine-loop".into())
            .spawn(move || engine_loop(worker_rt, worker_ctl, event_tx))
            .map_err(|e| format!("engine thread: {e}"))?;
        Ok(EngineClient {
            runtime,
            control,
            event_rx,
            join: Some(join),
        })
    }

    pub fn push_snapshot(&self, graph: Graph, time: CookContext, hint: TickHint) {
        if let Ok(mut rt) = self.runtime.lock() {
            rt.graph = graph;
            rt.time = time;
        }
        if let Ok(mut h) = self.control.hint.lock() {
            *h = hint;
        }
    }

    pub fn poll_frame(&self) -> Option<FrameEvent> {
        self.event_rx.try_recv().ok()
    }

    pub fn read_top_rgba8(&self, id: NodeId) -> Result<(u32, u32, Vec<u8>), String> {
        let rt = self
            .runtime
            .lock()
            .map_err(|e| format!("runtime lock: {e}"))?;
        rt.read_top_rgba8(id)
    }

    pub fn runtime(&self) -> &Arc<Mutex<ProjectRuntime>> {
        &self.runtime
    }

    pub fn top_output_meta(&self, id: NodeId) -> Option<(u32, u32, u64)> {
        let rt = self.runtime.lock().ok()?;
        let tex = rt.engines.top.output(&rt.graph, id)?;
        Some((tex.key.width, tex.key.height, tex.generation))
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
        self.runtime
            .lock()
            .ok()
            .and_then(|rt| rt.engines.python_error())
    }

    pub fn cook_frame_cost_ms(&self, id: NodeId) -> f64 {
        self.runtime
            .lock()
            .map(|rt| rt.cook.frame_cost_ms(id))
            .unwrap_or(0.0)
    }

    pub fn cook_avg_ms(&self, id: NodeId) -> f64 {
        self.runtime
            .lock()
            .map(|rt| rt.cook.avg_cook_ms(id))
            .unwrap_or(0.0)
    }

    pub fn cook_rate(&self, id: NodeId) -> f64 {
        self.runtime
            .lock()
            .map(|rt| rt.cook.cook_rate(id))
            .unwrap_or(0.0)
    }

    pub fn is_time_dependent(&self, id: NodeId) -> bool {
        self.runtime
            .lock()
            .map(|rt| rt.cook.is_time_dependent(id))
            .unwrap_or(false)
    }

    pub fn last_cook_us(&self, id: NodeId) -> u64 {
        self.runtime
            .lock()
            .map(|rt| rt.cook.last_cook_us(id))
            .unwrap_or(0)
    }

    pub fn has_cooked(&self, id: NodeId) -> bool {
        self.runtime
            .lock()
            .map(|rt| rt.cook.has_cooked(id))
            .unwrap_or(false)
    }

    pub fn cooked_this_frame(&self, id: NodeId, frame: i64) -> bool {
        self.runtime
            .lock()
            .map(|rt| rt.cook.cooked_this_frame(id, frame))
            .unwrap_or(false)
    }

    /// Bumped whenever this node's cook output changes — use for UI texture refresh.
    pub fn output_version(&self, id: NodeId) -> u64 {
        self.runtime
            .lock()
            .map(|rt| rt.cook.output_version(id))
            .unwrap_or(0)
    }

    pub fn top_gpu_stats(&self) -> (u32, u64, u64, u64) {
        self.runtime
            .lock()
            .map(|rt| {
                (
                    rt.engines.top.passes_this_frame,
                    rt.engines.top.resident_bytes(),
                    rt.engines.top.pooled_bytes(),
                    rt.engines.top.textures_created(),
                )
            })
            .unwrap_or((0, 0, 0, 0))
    }
}

impl Drop for EngineClient {
    fn drop(&mut self) {
        self.control.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn engine_loop(
    runtime: Arc<Mutex<ProjectRuntime>>,
    control: Arc<LoopControl>,
    event_tx: std::sync::mpsc::Sender<FrameEvent>,
) {
    let mut last = Instant::now();
    while !control.stop.load(Ordering::Relaxed) {
        let hint = control.hint.lock().map(|h| h.clone()).unwrap_or_default();
        let fps = hint.fps.max(1.0);
        let target = Duration::from_secs_f64(1.0 / fps);

        let now = Instant::now();
        let dt = (now - last).as_secs_f64().max(1e-6);
        last = now;

        let event = {
            let Ok(mut rt) = runtime.lock() else {
                continue;
            };
            rt.graph.clone_from(&hint.graph);
            rt.time = hint.time;
            rt.graph_shared.refresh_generation();
            let mut playback = hint.playback;
            let result = rt.tick_frame(
                FrameConfig {
                    dt,
                    roots: hint.roots.clone(),
                    audio_latency_ms: hint.audio_latency_ms,
                    input: hint.input.clone(),
                    display_blits: hint.display_blits.clone(),
                    defer_gpu_submit: hint.defer_gpu_submit,
                    max_time_slice_ms: hint.max_time_slice_ms,
                    advance_timeline: hint.advance_timeline,
                },
                &mut playback,
            );
            FrameEvent {
                time: rt.time,
                playback,
                result,
            }
        };

        let _ = event_tx.send(event);

        let elapsed = last.elapsed();
        if elapsed < target {
            thread::sleep(target - elapsed);
        }
    }
    if let Ok(mut rt) = runtime.lock() {
        rt.engines.shutdown();
    }
}
