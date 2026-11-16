pub const TYPE_NAME: &str = "reaperoutCHOP";

use super::{mark_rt_audio_sink, spec_animated_rt, ChopSpec, RT_AUDIO_SINK_REAPER};

pub fn spec() -> ChopSpec {
    mark_rt_audio_sink(
        spec_animated_rt(
            TYPE_NAME,
            "Reaper Out",
            &["in"],
            "Sends its input as ReaStream UDP audio (float32) for the Reaper ReaStream plugin.",
            crate::io::params_reaper_out,
            crate::io::cook_reaper_out,
        ),
        RT_AUDIO_SINK_REAPER,
    )
}
