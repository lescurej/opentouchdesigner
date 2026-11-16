use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) static PROGRAM_START_UNIX: LazyLock<f64> = LazyLock::new(|| {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
});

pub(crate) const SECS_PER_DAY: f64 = 86_400.0;
pub(crate) const SECS_PER_HOUR: f64 = 3_600.0;
pub(crate) const SECS_PER_MIN: f64 = 60.0;
pub(crate) const DAYS_PER_YEAR: f64 = 365.25;
