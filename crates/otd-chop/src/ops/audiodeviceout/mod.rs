pub const TYPE_NAME: &str = "audiodeviceoutCHOP";

use super::{mark_rt_audio_sink, spec_animated, ChopSpec, RT_AUDIO_SINK_DEVICE};

pub fn spec() -> ChopSpec {
    let mut s = mark_rt_audio_sink(
        spec_animated(
            TYPE_NAME,
            "Audio Device Out",
            &["in"],
            "Plays its input on an audio output; each CHOP channel maps to the matching device channel.",
            crate::io::params_audio_out,
            crate::io::cook_audio_out,
        ),
        RT_AUDIO_SINK_DEVICE,
    );
    s.rt_safe = true;
    s
}
