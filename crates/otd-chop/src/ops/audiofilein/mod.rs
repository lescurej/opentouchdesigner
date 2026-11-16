pub const TYPE_NAME: &str = "audiofileinCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Audio File In",
        &[],
        "Plays an audio file, following the timeline.",
        crate::io::params_audio_file,
        crate::io::cook_audio_file,
    )
}
