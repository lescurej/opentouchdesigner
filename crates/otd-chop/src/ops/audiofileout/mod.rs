pub const TYPE_NAME: &str = "audiofileoutCHOP";

use super::{spec_animated, ChopSpec};

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "Audio File Out",
        &["in"],
        "Records its input to a WAV file while Record is on. Passes the audio through.",
        crate::io::params_audio_file_out,
        crate::io::cook_audio_file_out,
    )
}
