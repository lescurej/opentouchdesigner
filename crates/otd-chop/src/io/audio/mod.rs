//! Device audio I/O: in, file, out, spectrum, ReaStream.

mod file;
mod file_out;
mod in_device;
mod movie;
mod out;
mod reastream;
mod spectrum;
mod stream;

pub(crate) use file::{AudioFile, cook_audio_file, params_audio_file};
pub(crate) use file_out::{AudioFileRec, cook_audio_file_out, params_audio_file_out};
pub(crate) use in_device::{AudioIn, cook_audio_in, params_audio_in};
pub use movie::AUDIO_MOVIE;
pub(crate) use out::{AudioOut, cook_audio_out, params_audio_out};
pub(crate) use reastream::{ReaStreamOut, cook_reaper_out, params_reaper_out};
pub(crate) use spectrum::{Spectrum, cook_spectrum, params_spectrum};

pub use stream::{
    AUDIO_LATENCY_DEFAULT_MS, AUDIO_LATENCY_MAX_MS, AUDIO_LATENCY_MIN_MS, clamp_audio_latency_ms,
};
#[cfg(test)]
pub(crate) use stream::{device_rate, latency_samples, rate_in_spans, rate_mismatch_note};
