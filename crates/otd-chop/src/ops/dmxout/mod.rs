pub const TYPE_NAME: &str = "dmxoutCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "DMX Out",
        &["in"],
        "Sends its channels as DMX over Art-Net or sACN.",
        crate::io::params_dmx_out,
        crate::io::cook_dmx_out,
    )
}
