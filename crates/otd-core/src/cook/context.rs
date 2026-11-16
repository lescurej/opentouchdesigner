use crate::expr::{ChannelSource, EvalContext};
use crate::graph::{Graph, NodeId};

/// Time state for one frame.
#[derive(Clone, Copy, Debug)]
pub struct CookContext {
    pub frame: i64,
    /// Component-local time, seconds.
    pub time: f64,
    /// Absolute time since the project started, seconds.
    pub abs_time: f64,
    pub fps: f64,
    /// Audio device clock, one of 44.1 / 48 / 96 / 192 kHz. CHOPs keep their
    /// own rates; this is the rate the speakers and the input open at.
    pub sample_rate: f64,
    /// Seconds elapsed since the previous frame.
    ///
    /// CHOPs need the *actual* frame interval, not the nominal one: TD-style
    /// time slicing generates however many samples cover the time that really
    /// passed, so control and audio stay continuous across a dropped frame
    /// (PLAN.md §4).
    pub dt: f64,
}

impl Default for CookContext {
    fn default() -> Self {
        let fps = crate::project::DEFAULT_PROJECT_FPS;
        CookContext {
            frame: 0,
            time: 0.0,
            abs_time: 0.0,
            fps,
            sample_rate: crate::project::PROJECT_SAMPLE_RATE_DEFAULT,
            dt: 1.0 / fps,
        }
    }
}

impl CookContext {
    /// Nominal seconds per project frame. [`Self::dt`] is the *last* frame's
    /// wall interval and may differ when the host drops frames or runs faster
    /// than the project (e.g. 120 Hz display, 60 fps timeline).
    pub fn nominal_dt(&self) -> f64 {
        1.0 / self.fps.max(1e-6)
    }

    /// Step the timeline by one nominal project frame (ideal pacing).
    pub fn advance_nominal_frame(&mut self) {
        self.advance(self.nominal_dt());
    }
    pub fn eval_ctx(&self) -> EvalContext<'static> {
        EvalContext {
            frame: self.frame,
            time: self.time,
            abs_time: self.abs_time,
            fps: self.fps,
            channels: None,
            path: None,
            locals: &[],
        }
    }

    /// The same context, with somewhere to resolve Export and Bind
    /// parameters from.
    pub fn eval_ctx_with<'a>(&self, channels: &'a dyn ChannelSource) -> EvalContext<'a> {
        EvalContext {
            channels: Some(channels),
            ..self.eval_ctx()
        }
    }

    pub fn eval_for_node<'a>(
        &self,
        graph: &Graph,
        id: NodeId,
        channels: &'a dyn ChannelSource,
        path: &'a str,
    ) -> EvalContext<'a> {
        let (time, abs_time, frame) = graph.local_clock_at(id, self);
        EvalContext {
            frame,
            time,
            abs_time,
            fps: self.fps,
            channels: Some(channels),
            path: Some(path),
            locals: &[],
        }
    }

    /// Advance by one frame of wall time.
    pub fn advance(&mut self, dt: f64) {
        self.time += dt;
        self.abs_time += dt;
        self.dt = dt;
        self.frame = (self.time * self.fps).round() as i64;
    }
}
