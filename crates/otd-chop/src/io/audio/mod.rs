//! Device audio I/O: in, file, out, spectrum, ReaStream.


mod stream;
mod in_device;
mod file;
mod file_out;
mod movie;
mod out;
mod spectrum;
mod reastream;

pub(crate) use in_device::{cook_audio_in, params_audio_in, AudioIn};
pub(crate) use file::{cook_audio_file, params_audio_file, AudioFile};
pub(crate) use file_out::{cook_audio_file_out, params_audio_file_out, AudioFileRec};
pub use movie::AUDIO_MOVIE;
pub(crate) use out::{cook_audio_out, params_audio_out, AudioOut};
pub(crate) use spectrum::{cook_spectrum, params_spectrum, Spectrum};
pub(crate) use reastream::{cook_reaper_out, params_reaper_out, ReaStreamOut};

pub use stream::{
    clamp_audio_latency_ms, AUDIO_LATENCY_DEFAULT_MS, AUDIO_LATENCY_MAX_MS, AUDIO_LATENCY_MIN_MS,
};
#[cfg(test)]
pub(crate) use stream::{device_rate, latency_samples, rate_in_spans, rate_mismatch_note};
pub use reastream::ensure_rt_reaper_sender;
