use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread;

use cidre::{arc, av, cf, cm, cv, ns};

use super::pixel::pixel_buf_to_frame;
use super::track::load_video_track;
use super::util::ns_error;
use super::super::Frame;

pub(crate) struct MacFileDecodeHold {
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) reader: Arc<Mutex<Option<arc::R<av::asset::Reader>>>>,
    pub(crate) io: Arc<Mutex<()>>,
}

impl MacFileDecodeHold {
    pub(crate) fn cancel(&self) {
        self.stop.store(true, Ordering::SeqCst);
        let _io = self.io.lock().unwrap();
        if let Ok(mut slot) = self.reader.lock() {
            if let Some(mut r) = slot.take() {
                r.cancel_reading();
            }
        }
    }
}

pub(super) fn decode_file_loop(
    path: &str,
    fps: f64,
    start_index: i64,
    tx: &SyncSender<(i64, Arc<Frame>)>,
    stop: &AtomicBool,
    reader_slot: Arc<Mutex<Option<arc::R<av::asset::Reader>>>>,
    decode_io: Arc<Mutex<()>>,
) -> Result<(), String> {
    let url = cf::Url::with_file_path(Path::new(path)).ok_or("invalid file path")?;
    let asset = av::UrlAsset::with_url(url.as_ns(), None).ok_or("could not open media file")?;
    let track = load_video_track(&asset).ok_or("no video track in file")?;

    let mut reader = av::asset::Reader::with_asset(&asset).map_err(ns_error)?;
    let mut out = track_output(&track)?;
    out.set_always_copies_sample_data(false);
    reader.add_output(&out).map_err(ns_error)?;

    let start_secs = start_index as f64 / fps.max(1e-6);
    let start = cm::Time::with_secs(start_secs, 600);
    let duration = cm::Time::with_secs(86400.0 * 7.0, 600);
    reader.set_time_range(cm::TimeRange { start, duration });

    reader.start_reading().map_err(ns_error)?;
    if let Ok(mut slot) = reader_slot.lock() {
        *slot = Some(reader.clone());
    }

    let mut index = start_index.max(0);
    while !stop.load(Ordering::SeqCst) {
        let sample = {
            let _io = decode_io.lock().unwrap();
            if stop.load(Ordering::SeqCst) {
                break;
            }
            match out.next_sample_buf() {
                Ok(v) => v,
                Err(e) => {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    return Err(ns_error(e));
                }
            }
        };
        match sample {
            Some(mut sample) => {
                let _ = sample.pts();
                let Some(pb) = sample.image_buf_mut().map(|b| b.retained()) else {
                    continue;
                };
                let Some(frame) = pixel_buf_to_frame(&pb) else {
                    continue;
                };
                let frame = Arc::new(frame);
                while !stop.load(Ordering::Relaxed) {
                    match tx.try_send((index, frame.clone())) {
                        Ok(()) => break,
                        Err(std::sync::mpsc::TrySendError::Full(_)) => {
                            thread::sleep(std::time::Duration::from_millis(2));
                        }
                        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return Ok(()),
                    }
                }
                index += 1;
            }
            None => break,
        }
        if reader.status() == av::asset::ReaderStatus::Completed {
            break;
        }
        if reader.status() == av::asset::ReaderStatus::Failed {
            if stop.load(Ordering::SeqCst) {
                break;
            }
            return Err(reader
                .error()
                .map(|e| e.localized_description().to_string())
                .unwrap_or_else(|| "asset reader failed".into()));
        }
    }
    if let Ok(mut slot) = reader_slot.lock() {
        slot.take();
    }
    Ok(())
}

fn track_output(track: &av::asset::Track) -> Result<arc::R<av::asset::ReaderTrackOutput>, String> {
    let settings = ns::Dictionary::with_keys_values(
        &[cv::pixel_buffer_keys::pixel_format().as_ns()],
        &[cv::PixelFormat::_32_BGRA.as_ref()],
    );
    av::asset::ReaderTrackOutput::with_track(track, Some(&settings)).map_err(ns_error)
}