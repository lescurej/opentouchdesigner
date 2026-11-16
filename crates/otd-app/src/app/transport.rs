pub struct TransportState {
    pub playing: bool,
    pub playing_before_minimize: Option<bool>,
    pub loop_range: (f64, f64),
    pub looping: bool,
    pub(crate) scrubbing: bool,
    pub smoothed_fps: f64,
    pub smoothed_frame_ms: f64,
    pub smoothed_wall_tick_ms: f64,
    pub smoothed_cook_ms: f64,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            playing: false,
            playing_before_minimize: None,
            loop_range: (0.0, 10.0),
            looping: true,
            scrubbing: false,
            smoothed_fps: 60.0,
            smoothed_frame_ms: 16.0,
            smoothed_wall_tick_ms: 16.0,
            smoothed_cook_ms: 0.0,
        }
    }
}