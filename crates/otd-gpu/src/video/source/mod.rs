mod reader;
mod open;
mod playback;
mod advance;
mod ffmpeg;
#[cfg(test)]
mod test_util;

pub use reader::{MovieDecodeBackend, Source};
pub(crate) use reader::{Reader, QUEUE_DEPTH, SEEK_AHEAD_LIMIT, FIRST_FRAME_GRACE};
pub(crate) use ffmpeg::spawn_ffmpeg;

#[cfg(test)]
pub(crate) use test_util::{test_source, test_source_with_reader};