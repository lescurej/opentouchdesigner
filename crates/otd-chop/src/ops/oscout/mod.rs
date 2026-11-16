pub const TYPE_NAME: &str = "oscoutCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "OSC Out",
        &["in"],
        "Sends each channel as an OSC message to the network address and port.",
        crate::io::params_osc_out,
        crate::io::cook_osc_out,
    )
}
