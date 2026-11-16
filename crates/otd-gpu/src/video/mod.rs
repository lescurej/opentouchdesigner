//! Video and image input: Movie File In and Video Device In.
//!
//! PLAN.md Phase 1 asks for these on GStreamer. GStreamer is a *system*
//! dependency rather than a crate — it has to be installed, versioned and
//! shipped per platform — and a build that cannot be compiled or verified is
//! not worth writing. §3 of the plan already sanctions the alternative:
//! "ffmpeg subprocess for offline export". This module takes that seriously
//! as the transport for input too.
//!
//! So there are two paths, chosen by what the file is:
//!
//!  * **Still images** — PNG, JPEG, WebP, BMP, TGA, TIFF — are decoded
//!    in-process by the `image` crate. No external tool, no way for it to be
//!    missing, and no subprocess for the case where you just want one picture.
//!  * **Everything that moves** — mp4, mov, mkv, webm, avi, gif — and every
//!    camera runs through an `ffmpeg` subprocess piping raw RGBA on stdout.
//!    If ffmpeg is not installed the node says so, by name, and produces
//!    black rather than failing the cook.
//!
//! There is no one cross-platform “native” movie API: macOS has AVFoundation,
//! Windows has Media Foundation, Linux usually GStreamer. A future in-process
//! backend per OS is possible; today ffmpeg (on PATH or bundled next to the app)
//! is the portable choice that matches what we ship in `package-macos.sh`.
//!
//! What this trades away against GStreamer is in-process zero-copy hardware
//! decode: a pipe costs a memcpy per frame and an upload. What it buys is
//! that it works on all three platforms today, with no build-time dependency
//! and nothing to install to *compile* OpenTouchDesigner.
//!
//! Two rules from the rest of the codebase apply unchanged. Decoding happens
//! on its own thread and hands finished frames to the cook, so nothing here
//! can block a frame. And a missing file, missing camera or missing ffmpeg
//! degrades to a message on the node, because losing a source mid-show must
//! not stop the render.
//!
//! On macOS, movie files are decoded in-process with AVFoundation (cidre);
//! ffmpeg remains the fallback when that path fails and on other platforms.


mod decode;
mod device;
mod source;

#[cfg(target_os = "macos")]
mod macos;

use std::sync::{Arc, Mutex};

/// One decoded frame, RGBA8, tightly packed.
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// Soundtrack for one movie file. The worker fills it; cooks only clone the `Arc`.
pub(crate) struct MovieAudioHandle {

    pub path: String,
    pub audio: Arc<Mutex<otd_core::MovieAudio>>,
}

/// Start reading the file's audio track on the movie's own timeline.
///
/// On macOS this is AVFoundation, the same framework that decodes the picture,
/// so sample 0 is the same media time as frame 0. Other platforms have no
/// AVFoundation reader.
pub(crate) fn spawn_movie_audio(path: String) -> MovieAudioHandle {
    let audio = Arc::new(Mutex::new(otd_core::MovieAudio::Pending));
    #[cfg(target_os = "macos")]
    {
        let publish = audio.clone();
        let file = path.clone();
        let _ = std::thread::Builder::new()
            .name("otd-movie-audio".into())
            .spawn(move || {
                if let Err(e) = macos::read_soundtrack(&file, &publish) {
                    let next = if e == "no audio" {
                        otd_core::MovieAudio::Missing
                    } else {
                        otd_core::MovieAudio::Failed(e)
                    };
                    *publish.lock().unwrap_or_else(|err| err.into_inner()) = next;
                }
            });
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(mut slot) = audio.lock() {
            *slot = otd_core::MovieAudio::Failed(
                "movie audio is read through AVFoundation, which is macOS only".into(),
            );
        }
    }
    MovieAudioHandle { path, audio }
}

pub use decode::{is_still, missing_ffmpeg, probe, tools_installed, Info, position_from_units};
pub use device::{camera_input, camera_modes, Mode};
pub use source::Source;
pub use source::MovieDecodeBackend;
pub(crate) use source::Reader;

#[cfg(test)]
pub(crate) use decode::{decode_still, parse_rational};
#[cfg(test)]
pub(crate) use device::{choose_mode, parse_mode_line};

#[cfg(test)]
#[path = "../tests/video_mod__tests.rs"]
mod tests;
