use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, TrySendError, SyncSender};
use std::sync::Arc;

use super::super::Frame;
use super::reader::{MovieDecodeBackend, Reader, SharedChild};

/// Start ffmpeg writing raw RGBA to stdout and a thread reading frames off it.
pub(crate) fn spawn_ffmpeg(
    mut command: Command,
    width: u32,
    height: u32,
    start_index: i64,
    problem: Arc<std::sync::Mutex<Option<String>>>,
    queue_depth: usize,
) -> Result<Reader, String> {
    if width == 0 || height == 0 {
        return Err("source has no picture size".into());
    }
    let mut child = command
        .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
        .stdout(Stdio::piped())
        // Kept rather than discarded: when a capture device refuses a mode,
        // ffmpeg's own words list the ones it would accept, and that belongs
        // on the node where somebody can read it.
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "ffmpeg is not installed — `brew install ffmpeg`, or use a PNG/JPEG still".into()
            }
            _ => format!("could not start ffmpeg: {e}"),
        })?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or("ffmpeg produced no output pipe")?;
    if let Some(mut stderr) = child.stderr.take() {
        let sink = problem.clone();
        let _ = std::thread::Builder::new()
            .name("otd-video-stderr".into())
            .spawn(move || {
                let mut text = String::new();
                use std::io::Read as _;
                let _ = stderr.read_to_string(&mut text);
                let text = text.trim();
                if !text.is_empty() {
                    if let Ok(mut slot) = sink.lock() {
                        *slot = Some(text.to_string());
                    }
                }
            });
    }
    let child = Arc::new(SharedChild::new(child));
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, frames): (SyncSender<(i64, Arc<Frame>)>, _) =
        sync_channel(queue_depth.clamp(1, 120));

    let worker_stop = stop.clone();
    let worker = std::thread::Builder::new()
        .name("otd-video-decode".into())
        .spawn(move || {
            let bytes = width as usize * height as usize * 4;
            let mut index = start_index;
            let mut buffer = vec![0u8; bytes];
            while !worker_stop.load(Ordering::Relaxed) {
                if stdout.read_exact(&mut buffer).is_err() {
                    break; // End of stream, or the process was killed.
                }
                let pixels = std::mem::replace(&mut buffer, vec![0u8; bytes]);
                let frame = Arc::new(Frame {
                    width,
                    height,
                    pixels,
                });
                // Block when the cook is behind: back-pressure is what keeps
                // a fast source from filling memory. Only a disconnect ends
                // the loop — a full queue is normal.
                let mut pending = (index, frame);
                loop {
                    match tx.try_send(pending) {
                        Ok(()) => break,
                        Err(TrySendError::Full(back)) => {
                            pending = back;
                            if worker_stop.load(Ordering::Relaxed) {
                                return;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(2));
                        }
                        Err(TrySendError::Disconnected(_)) => return,
                    }
                }
                index += 1;
            }
        })
        .map_err(|e| format!("could not start ffmpeg decode thread: {e}"))?;

    Ok(Reader {
        frames,
        stop,
        worker: Some(worker),
        child: Some(child),
        file_backend: Some(MovieDecodeBackend::Ffmpeg),
        #[cfg(target_os = "macos")]
        capture_hold: None,
        #[cfg(target_os = "macos")]
        file_decode_hold: None,
    })
}
