mod calendar;
mod common;
mod cook;
mod params;
mod solar;

use super::{spec_animated, ChopSpec};

pub const TYPE_NAME: &str = "clockCHOP";

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Clock",
        &["target", "reference"],
        "Wall-clock time, which keeps running when the timeline is paused.",
        params::params_clock,
        cook::cook_clock,
    )
}
