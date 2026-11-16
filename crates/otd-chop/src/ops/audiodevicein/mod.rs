pub const TYPE_NAME: &str = "audiodeviceinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Audio Device In",
        &[],
        "Samples from an audio input, downmixed to mono.",
        crate::io::params_audio_in,
        crate::io::cook_audio_in,
    )
}
