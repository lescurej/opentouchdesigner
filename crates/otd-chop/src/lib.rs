//! `otd-chop` — the CHOP (channel operator) family.
//!
//! Channels of samples: the control and audio side of the network. No GPU, no
//! UI; device work happens on its own threads and hands buffers to the cook
//! (PLAN.md §4).

pub mod anim;
pub mod bclip;
pub mod chopfile;
pub mod data;
mod perform;
pub mod dmx;
pub mod engine;
pub mod io;
pub mod ops;
pub mod rt;
mod oscvoice;
mod reastream;
pub use otd_audio::thread_qos;
mod timecode;
mod tracking;
pub mod wav;

pub use anim::{Curve, Curves, Interp, Key};
pub use data::{CONTROL_RATE, Channel, ChopData};
pub use perform::PerformFrameStats;
pub use engine::{ChannelStore, ChopEngine, ChopHost, Network};
pub use io::{
    AUDIO_LATENCY_DEFAULT_MS, AUDIO_LATENCY_MAX_MS, AUDIO_LATENCY_MIN_MS, InputState, Io,
    PanelInteractionSample, PanelInteractionStore, PerformPanelInput, RecordBuffer,
    clamp_audio_latency_ms,
};
pub use timecode::chop_timecode_frame_index;

pub fn register_all(registry: &mut otd_core::OpRegistry) {
    ops::register_all(registry);
}
