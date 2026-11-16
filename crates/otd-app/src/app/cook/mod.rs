use super::*;

mod frame;
mod perform;
mod roots;
mod thumbs;


const VIEWER_THUMB_MAX: u32 = 320;
const VIEWER_THUMB_MAX_PLAYING: u32 = 192;

impl OtdApp {
    pub(crate) fn sync_viewport_playback(&mut self, ctx: &egui::Context) {
        if !self.settings.stop_playing_when_minimized {
            self.transport.playing_before_minimize = None;
            return;
        }
        let minimized = ctx.input(|i| i.viewport().minimized == Some(true));
        if minimized {
            if self.transport.playing_before_minimize.is_none() && self.transport.playing {
                self.transport.playing_before_minimize = Some(true);
                self.transport.playing = false;
            }
        } else if let Some(was_playing) = self.transport.playing_before_minimize.take() {
            if was_playing {
                self.transport.playing = true;
            }
        }
    }

    pub(super) fn wants_live_thumbnail(&self, id: NodeId) -> bool {
        if !self.transport.playing {
            return true;
        }
        if self.canvas.visible.contains(&id) {
            return true;
        }
        if let Some(top) = crate::window_comp::perform_display_top(
            &self.graph,
            self.perform_shell.main_perform_window,
            self.perform_ui(),
        ) && top == id
        {
            return true;
        }
        for vid in &self.canvas.visible {
            if self.graph.node(*vid).family != otd_core::Family::Comp
                || !self.graph.node(*vid).flags.viewer
            {
                continue;
            }
            if crate::canvas::comp_display::comp_view_top(&self.graph, *vid) == Some(id) {
                return true;
            }
        }
        false
    }

    pub(crate) fn audio_latency_ms(&self) -> u32 {
        self.settings.audio_latency_ms
    }
}

