mod ltc;
mod smpte;

pub use ltc::*;
pub use otd_core::movie_timecode_frame_index as chop_timecode_frame_index;
pub use smpte::*;

#[cfg(test)]
#[path = "../tests/timecode__tests.rs"]
mod tests;
