use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use otd_chop::InputState;
use otd_core::{CookContext, Graph, NodeId};

fn should_resync_time(rt: &CookContext, snap: &CookContext) -> bool {
    snap.abs_time > rt.abs_time + 0.001
}

use crate::{FrameConfig, FrameResult, PlaybackState, ProjectRuntime};

#[derive(Clone)]
pub struct CookHint {
    pub graph: Graph,
    pub time: CookContext,
    pub roots: Vec<NodeId>,
    pub playback: PlaybackState,
    pub input: InputState,
    pub fps: f64,
    pub audio_latency_ms: u32,
    pub defer_gpu_submit: bool,
    pub max_time_slice_ms: f64,
    pub advance_timeline: bool,
}

impl Default for CookHint {
    fn default() -> Self {
        CookHint {
            graph: Graph::new(),
            time: CookContext::default(),
            roots: Vec::new(),
            playback: PlaybackState::default(),
            input: InputState::default(),
            fps: otd_core::project::DEFAULT_PROJECT_FPS,
            audio_latency_ms: 0,
            defer_gpu_submit: false,
            max_time_slice_ms: otd_core::project::DEFAULT_MAX_TIME_SLICE_MS,
            advance_timeline: true,
        }
    }
}

pub struct CookReport {
    pub result: FrameResult,
    pub time: CookContext,
    pub scrubbing: bool,
}

pub fn background_cook_loop(
    runtime: Arc<Mutex<ProjectRuntime>>,
    stop: Arc<AtomicBool>,
    ui_owns: Arc<AtomicBool>,
    hint: Arc<Mutex<CookHint>>,
    reports: Arc<Mutex<Vec<CookReport>>>,
) {
    otd_chop::thread_qos::promote_current_thread_interactive();
    let mut last = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        if ui_owns.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        let now = Instant::now();
        let wall_dt = (now - last).as_secs_f64().clamp(1e-6, 0.25);
        let snapshot = hint.lock().expect("cook hint").clone();
        let fps = snapshot.fps.max(1.0);
        let report = {
            let Ok(mut rt) = runtime.lock() else {
                break;
            };
            if stop.load(Ordering::Relaxed) || ui_owns.load(Ordering::Acquire) {
                None
            } else {
                rt.graph = snapshot.graph.clone();
                if should_resync_time(&rt.time, &snapshot.time) {
                    rt.time = snapshot.time;
                }
                let mut playback = snapshot.playback;
                let result = rt.tick_frame(
                    FrameConfig {
                        dt: wall_dt,
                        roots: snapshot.roots,
                        audio_latency_ms: snapshot.audio_latency_ms,
                        input: snapshot.input,
                        display_blits: Vec::new(),
                        defer_gpu_submit: snapshot.defer_gpu_submit,
                        max_time_slice_ms: snapshot.max_time_slice_ms,
                        advance_timeline: snapshot.advance_timeline,
                    },
                    &mut playback,
                );
                let report = CookReport {
                    result,
                    time: rt.time,
                    scrubbing: playback.scrubbing,
                };
                if let Ok(mut h) = hint.lock() {
                    h.time = rt.time;
                }
                Some(report)
            }
        };
        let Some(report) = report else {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        last = now;
        {
            let mut queue = reports.lock().expect("cook reports");
            if queue.len() == 32 {
                queue.remove(0);
            }
            queue.push(report);
        }
        let frame = Duration::from_secs_f64(1.0 / fps);
        let elapsed = last.elapsed();
        if elapsed < frame {
            std::thread::sleep(frame - elapsed);
        }
    }
}

#[cfg(test)]
#[path = "tests/background_cook__tests.rs"]
mod tests;
