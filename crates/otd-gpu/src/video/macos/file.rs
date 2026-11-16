use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

use cidre::{arc, av, cf};

use super::decode::{decode_file_loop, MacFileDecodeHold};
use super::track::load_video_track;
use super::super::{Frame, Info, MovieDecodeBackend, Reader};

pub fn probe(path: &str) -> Option<Info> {
    let url = cf::Url::with_file_path(Path::new(path))?;
    let asset = av::UrlAsset::with_url(url.as_ns(), None)?;
    let duration = asset.duration().as_secs();
    let track = load_video_track(&asset)?;
    let size = track.natural_size();
    let fps = track.nominal_frame_rate().max(1.0) as f64;
    Some(Info {
        width: size.width.max(1.0) as u32,
        height: size.height.max(1.0) as u32,
        fps,
        duration,
    })
}

pub fn spawn_file_reader(
    path: &str,
    info: &Info,
    start_index: i64,
    queue_depth: usize,
    problem: Arc<std::sync::Mutex<Option<String>>>,
) -> Result<Reader, String> {
    if info.width == 0 || info.height == 0 {
        return Err("source has no picture size".into());
    }
    let path = path.to_string();
    let fps = info.fps;
    let stop = Arc::new(AtomicBool::new(false));
    let reader_slot: Arc<Mutex<Option<arc::R<av::asset::Reader>>>> = Arc::new(Mutex::new(None));
    let decode_io = Arc::new(Mutex::new(()));
    let file_decode_hold = MacFileDecodeHold {
        stop: stop.clone(),
        reader: reader_slot.clone(),
        io: decode_io.clone(),
    };
    let queue = queue_depth.clamp(4, 64);
    let (tx, frames) = sync_channel::<(i64, Arc<Frame>)>(queue);
    let worker_stop = stop.clone();
    let sink = problem.clone();
    let worker = thread::Builder::new()
        .name("otd-video-cidre".into())
        .spawn(move || {
            if let Err(e) = decode_file_loop(
                &path,
                fps,
                start_index,
                &tx,
                &worker_stop,
                reader_slot,
                decode_io,
            ) {
                if let Ok(mut slot) = sink.lock() {
                    if slot.is_none() {
                        *slot = Some(e);
                    }
                }
            }
        })
        .map_err(|e| format!("could not start video decode thread: {e}"))?;
    Ok(Reader {
        frames,
        stop,
        worker: Some(worker),
        child: None,
        file_backend: Some(MovieDecodeBackend::AvFoundation),
        capture_hold: None,
        file_decode_hold: Some(file_decode_hold),
    })
}