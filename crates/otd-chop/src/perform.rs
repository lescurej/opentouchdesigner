#[derive(Clone, Copy, Debug, Default)]
pub struct PerformFrameStats {
    pub cookrate: f64,
    pub fps: f64,
    pub msec: f64,
    pub timeslicestep: f64,
    pub timeslicemsec: f64,
}

impl PerformFrameStats {
    pub fn from_cook(time: &otd_core::CookContext, cook_ms: f64, interval_ms: f64) -> Self {
        let timeslicemsec = time.dt * 1000.0;
        let timeslicestep = (time.dt * time.fps.max(1e-6)).max(1.0);
        let interval_ms = if interval_ms > 1e-3 {
            interval_ms
        } else {
            time.nominal_dt() * 1000.0
        };
        let fps = 1000.0 / interval_ms;
        PerformFrameStats {
            cookrate: time.fps,
            fps,
            msec: cook_ms,
            timeslicestep,
            timeslicemsec,
        }
    }
}
