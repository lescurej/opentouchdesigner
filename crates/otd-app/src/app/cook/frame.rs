
use crate::engine_ui::playback_from_app;
use otd_core::NodeId;
use otd_engine::TickHint;

use super::{VIEWER_THUMB_MAX, VIEWER_THUMB_MAX_PLAYING};

impl super::super::OtdApp {
    pub(crate) fn cook_frame(
        &mut self,
        ctx: &egui::Context,
        include_editor: bool,
        advance_timeline: bool,
    ) {
        let roots = self.cook_roots(include_editor);
        let _ = ctx;
        let hint = TickHint {
            graph: self.graph.clone(),
            time: self.time,
            roots,
            playback: playback_from_app(
                self.transport.playing,
                self.transport.looping,
                self.transport.scrubbing,
                self.transport.loop_range,
            ),
            input: self.input_state.clone(),
            audio_latency_ms: self.audio_latency_ms(),
            fps: self.time.fps,
            display_blits: Vec::new(),
            defer_gpu_submit: self.transport.playing,
            max_time_slice_ms: self.settings.max_time_slice_ms,
            advance_timeline,
        };
        self.frame_perf_probe.sync_blits = 0;
        let (result, playback) = self
            .engine
            .cook_frame(&mut self.graph, &mut self.time, hint);
        self.transport.scrubbing = playback.scrubbing;
        self.apply_engine_frame(&result);
    }

    pub(super) fn take_texture_sync_batch(&mut self) -> Vec<NodeId> {
        let thumb_cap = if self.transport.playing {
            VIEWER_THUMB_MAX_PLAYING
        } else {
            VIEWER_THUMB_MAX
        };
        let _ = thumb_cap;
        let mut pending = std::mem::take(&mut self.pending_texture_sync);
        if self.transport.playing {
            let (live, rest): (Vec<_>, Vec<_>) = pending
                .into_iter()
                .partition(|id| self.wants_live_thumbnail(*id));
            self.pending_texture_sync = rest;
            pending = live;
            pending.sort_by_key(|id| self.canvas.visible.contains(id));
        }
        let batch = std::mem::take(&mut pending);
        self.pending_texture_sync.extend(pending);
        batch
    }

    pub(crate) fn apply_engine_frame(&mut self, frame_result: &otd_engine::FrameResult) {
        self.last_frame_timing = frame_result.timing;

        if frame_result.graph_sync.replicated > 0 {
            self.document.status = format!("replicated {} node(s)", frame_result.graph_sync.replicated);
            self.repair_app_graph_pointers();
        } else if frame_result.graph_sync.clones_synced > 0 {
            self.document.status = format!("synced {} clone(s)", frame_result.graph_sync.clones_synced);
            self.repair_app_graph_pointers();
        }

        self.document.cook_error = frame_result.cook_error.clone();
        let frame = self.time.frame;
        if let Some(err) = &self.document.cook_error {
            if self.textport.last_cook_error.as_deref() != Some(err.as_str()) {
                self.textport.error(frame, err.clone());
                self.textport.last_cook_error = Some(err.clone());
            }
        } else {
            self.textport.last_cook_error = None;
        }

        let cb = &frame_result.callbacks;
        for p in &cb.param_edit_problems {
            self.textport.warn(frame, p);
        }
        if let Some(first) = cb.param_edit_problems.first() {
            self.document.status = format!("callback: {first}");
        } else if cb.param_edits_applied > 0 {
            self.document.status = format!("callback set {} parameter(s)", cb.param_edits_applied);
            self.textport.info(
                frame,
                format!(
                    "callback applied {} parameter edit(s)",
                    cb.param_edits_applied
                ),
            );
        }
        for line in &cb.script_log {
            self.textport.info(frame, line);
        }

        let ms = frame_result.timing.cook_ms;
        self.transport.smoothed_cook_ms = self.transport.smoothed_cook_ms * 0.92 + ms * 0.08;
        if !self.transport.playing {
            let interval = frame_result.timing.interval_ms;
            let instant_fps = 1000.0 / interval.max(0.001);
            self.transport.smoothed_fps = self.transport.smoothed_fps * 0.92 + instant_fps * 0.08;
            self.transport.smoothed_frame_ms = 1000.0 / self.transport.smoothed_fps.max(0.001);
        }
        let snap_started = std::time::Instant::now();
        self.rebuild_engine_ui_snapshot();
        self.frame_perf_probe.snapshot_ms = snap_started.elapsed().as_secs_f64() * 1000.0;
        self.frame_perf_probe.cook_ms = frame_result.timing.cook_ms;
        self.frame_perf_probe.cook_wall_ms = frame_result.timing.wall_ms;
    }
}
