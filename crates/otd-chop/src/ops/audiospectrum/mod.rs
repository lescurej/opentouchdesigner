pub const TYPE_NAME: &str = "audiospectrumCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Audio Spectrum",
        &["in"],
        "FFT magnitude (amplitude) and phase (degrees); linear frequency in hertz via sample index.",
        crate::io::params_spectrum,
        crate::io::cook_spectrum,
    )
}
