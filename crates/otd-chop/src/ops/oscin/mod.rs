pub const TYPE_NAME: &str = "oscinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "OSC In",
        &[],
        "Incoming OSC messages, one channel per address argument.",
        crate::io::params_osc_in,
        crate::io::cook_osc_in,
    )
}
