pub const TYPE_NAME: &str = "dmxinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "DMX In",
        &[],
        "Incoming DMX over Art-Net or sACN, one channel per slot.",
        crate::io::params_dmx_in,
        crate::io::cook_dmx_in,
    )
}
