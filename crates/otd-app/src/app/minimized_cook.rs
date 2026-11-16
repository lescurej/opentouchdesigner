use otd_engine::{
    CookHint, CookReport, ProjectRuntime, background_cook_loop, graph_audio_generation,
};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use super::OtdApp;

pub(super) struct BackgroundEngine {
    stop: Arc<AtomicBool>,
    ui_owns: Arc<AtomicBool>,
    hint: Arc<Mutex<CookHint>>,
    reports: Arc<Mutex<Vec<CookReport>>>,
    join: Option<JoinHandle<()>>,
    last_graph_gen: AtomicU64,
    include_editor: AtomicBool,
}

impl BackgroundEngine {
    pub(super) fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
            ui_owns: Arc::new(AtomicBool::new(true)),
            hint: Arc::new(Mutex::new(CookHint::default())),
            reports: Arc::new(Mutex::new(Vec::new())),
            join: None,
            last_graph_gen: AtomicU64::new(0),
            include_editor: AtomicBool::new(false),
        }
    }

    pub(super) fn is_paused_for_ui(&self) -> bool {
        self.join.is_some() && self.ui_owns.load(Ordering::Acquire)
    }

    pub(super) fn ensure_started(&mut self, runtime: Arc<Mutex<ProjectRuntime>>) {
        if self.join.is_some() {
            return;
        }
        self.stop.store(false, Ordering::Relaxed);
        self.ui_owns.store(true, Ordering::Release);
        let stop = self.stop.clone();
        let ui_owns = self.ui_owns.clone();
        let hint = self.hint.clone();
        let reports = self.reports.clone();
        let join = thread::Builder::new()
            .name("otd-cook".into())
            .spawn(move || background_cook_loop(runtime, stop, ui_owns, hint, reports))
            .expect("otd-cook thread");
        self.join = Some(join);
    }

    pub(super) fn release_to_background(&self) {
        self.ui_owns.store(false, Ordering::Release);
    }

    pub(super) fn pause_for_ui(&self) {
        self.ui_owns.store(true, Ordering::Release);
    }

    pub(super) fn publish(&self, app: &OtdApp) {
        let graph_gen = graph_audio_generation(&app.graph);
        let last = self.last_graph_gen.load(Ordering::Relaxed);
        let mut hint = self.hint.lock().expect("cook hint");
        if graph_gen != last {
            hint.graph = app.graph.clone();
            self.last_graph_gen.store(graph_gen, Ordering::Release);
        }
        hint.time = app.time;
        let include_editor = self.include_editor.load(Ordering::Relaxed);
        hint.roots = app.cook_roots(include_editor);
        hint.playback = crate::engine_ui::playback_from_app(
            app.transport.playing,
            app.transport.looping,
            app.transport.scrubbing,
            app.transport.loop_range,
        );
        hint.input = app.input_state.clone();
        hint.fps = app.time.fps;
        hint.audio_latency_ms = app.audio_latency_ms();
        hint.defer_gpu_submit = app.transport.playing;
        hint.max_time_slice_ms = app.settings.max_time_slice_ms;
        hint.advance_timeline = true;
    }

    fn drain(&self) -> Vec<CookReport> {
        std::mem::take(&mut *self.reports.lock().expect("cook reports"))
    }

    pub(super) fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.ui_owns.store(true, Ordering::Release);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
        self.ui_owns.store(false, Ordering::Relaxed);
        self.stop.store(false, Ordering::Relaxed);
        self.last_graph_gen.store(0, Ordering::Relaxed);
    }
}

impl OtdApp {
    pub(super) fn sync_background_cook(&mut self, minimized: bool) {
        self.background_engine
            .ensure_started(self.engine.runtime().clone());
        let include_editor = !minimized && !self.perform_preview_only();
        self.background_engine
            .include_editor
            .store(include_editor, Ordering::Relaxed);
        if self.transport.playing {
            self.drain_background_reports();
            self.background_engine.publish(self);
            self.background_engine.release_to_background();
        } else {
            self.background_engine.pause_for_ui();
            self.drain_background_reports();
        }
    }

    pub(super) fn drain_background_reports(&mut self) {
        let pending = self.background_engine.drain();
        if !pending.is_empty() {
            self.frame_perf_probe.did_cook = true;
        }
        for report in &pending {
            self.time = report.time;
            self.apply_engine_frame(&report.result);
        }
        if let Some(report) = pending.last() {
            self.transport.scrubbing = report.scrubbing;
        }
    }

    pub(super) fn shutdown_background_engine(&mut self) {
        self.background_engine.shutdown();
        self.playback_activity.set_playing(false);
    }
}
