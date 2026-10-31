use std::path::Path;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::thread;

use av::capture::VideoDataOutputSampleBufDelegate;
use cidre::{arc, av, cf, cm, cv, define_obj_type, dispatch, ns, objc};

use super::{Frame, Info, Reader};

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
    let reader_slot: Arc<Mutex<Option<arc::R<av::asset::Reader>>>> =
        Arc::new(Mutex::new(None));
    let decode_io = Arc::new(Mutex::new(()));
    let file_decode_hold = MacFileDecodeHold {
        stop: stop.clone(),
        reader: reader_slot.clone(),
        io: decode_io.clone(),
    };
    let queue = queue_depth.max(4).min(64);
    let (tx, frames): (SyncSender<(i64, Arc<Frame>)>, Receiver<(i64, Arc<Frame>)>) =
        sync_channel(queue);
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
        file_backend: Some(super::MovieDecodeBackend::AvFoundation),
        capture_hold: None,
        file_decode_hold: Some(file_decode_hold),
    })
}

fn load_video_track(asset: &av::UrlAsset) -> Option<arc::R<av::asset::Track>> {
    let (tx, rx) = std::sync::mpsc::channel();
    asset.load_tracks_with_media_type_block(av::MediaType::video(), move |tracks, err| {
        let track = if err.is_none() {
            tracks.and_then(|t| t.get(0).ok().map(|tr| tr.retained()))
        } else {
            None
        };
        let _ = tx.send(track);
    });
    rx.recv().ok().flatten()
}

pub(crate) struct MacFileDecodeHold {
    stop: Arc<AtomicBool>,
    reader: Arc<Mutex<Option<arc::R<av::asset::Reader>>>>,
    io: Arc<Mutex<()>>,
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

fn decode_file_loop(
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

fn pixel_buf_to_frame(pb: &cv::PixelBuf) -> Option<Frame> {
    let w = pb.width() as u32;
    let h = pb.height() as u32;
    if w == 0 || h == 0 {
        return None;
    }
    let fmt = pb.pixel_format();
    let mut pixels = vec![0u8; (w * h * 4) as usize];
    let lock = cv::pixel_buffer::LockFlags::DEFAULT;
    unsafe {
        let pb_mut = pb as *const cv::PixelBuf as *mut cv::PixelBuf;
        (*pb_mut).lock_base_addr(lock).result().ok()?;
        if fmt == cv::PixelFormat::_32_BGRA {
            copy_bgra_to_rgba(pb, w, h, &mut pixels);
        } else if fmt == cv::PixelFormat::_420F || fmt == cv::PixelFormat::_420V {
            copy_420_to_rgba(pb, w, h, fmt == cv::PixelFormat::_420F, &mut pixels);
        } else {
            let _ = (*pb_mut).unlock_lock_base_addr(lock);
            return None;
        }
        let _ = (*pb_mut).unlock_lock_base_addr(lock);
    }
    Some(Frame {
        width: w,
        height: h,
        pixels,
    })
}

fn copy_bgra_to_rgba(pb: &cv::PixelBuf, w: u32, h: u32, out: &mut [u8]) {
    let stride = pb.plane_bytes_per_row(0).max(w as usize * 4);
    let base = pb.plane_base_address(0);
    if !base.is_null() && pb.plane_count() >= 1 {
        for y in 0..h as usize {
            for x in 0..w as usize {
                let i = y * stride + x * 4;
                let o = (y * w as usize + x) * 4;
                unsafe {
                    out[o] = *base.add(i + 2);
                    out[o + 1] = *base.add(i + 1);
                    out[o + 2] = *base.add(i);
                    out[o + 3] = *base.add(i + 3);
                }
            }
        }
        return;
    }
    let base = pixel_base_address(pb);
    let stride = pixel_bytes_per_row(pb).max(w as usize * 4);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let i = y * stride + x * 4;
            let o = (y * w as usize + x) * 4;
            unsafe {
                out[o] = *base.add(i + 2);
                out[o + 1] = *base.add(i + 1);
                out[o + 2] = *base.add(i);
                out[o + 3] = *base.add(i + 3);
            }
        }
    }
}

fn copy_420_to_rgba(pb: &cv::PixelBuf, w: u32, h: u32, full_range: bool, out: &mut [u8]) {
    let y_plane = pb.plane_base_address(0);
    let uv_plane = pb.plane_base_address(1);
    if y_plane.is_null() || uv_plane.is_null() {
        return;
    }
    let y_stride = pb.plane_bytes_per_row(0);
    let uv_stride = pb.plane_bytes_per_row(1);
    for y in 0..h as usize {
        for x in 0..w as usize {
            let yo = y * y_stride + x;
            let uvo = (y / 2) * uv_stride + (x / 2) * 2;
            let yv = unsafe { *y_plane.add(yo) } as i32;
            let u = unsafe { *uv_plane.add(uvo) } as i32;
            let v = unsafe { *uv_plane.add(uvo + 1) } as i32;
            let (y_adj, u_adj, v_adj) = if full_range {
                (yv, u - 128, v - 128)
            } else {
                (yv - 16, u - 128, v - 128)
            };
            let yf = y_adj as f32 * 1.164;
            let r = (yf + 1.596 * v_adj as f32).clamp(0.0, 255.0) as u8;
            let g = (yf - 0.392 * u_adj as f32 - 0.813 * v_adj as f32).clamp(0.0, 255.0) as u8;
            let b = (yf + 2.017 * u_adj as f32).clamp(0.0, 255.0) as u8;
            let o = (y * w as usize + x) * 4;
            out[o] = r;
            out[o + 1] = g;
            out[o + 2] = b;
            out[o + 3] = 255;
        }
    }
}

fn pixel_base_address(pb: &cv::PixelBuf) -> *const u8 {
    unsafe { CVPixelBufferGetBaseAddress(pb) as *const u8 }
}

fn pixel_bytes_per_row(pb: &cv::PixelBuf) -> usize {
    unsafe { CVPixelBufferGetBytesPerRow(pb) }
}

fn ns_error<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

pub(crate) struct MacCaptureHold {
    session: arc::R<av::capture::Session>,
    _delegate: arc::R<CameraDelegate>,
    _queue: arc::R<dispatch::Queue>,
}

impl MacCaptureHold {
    pub(crate) fn stop(&mut self) {
        self.session.stop_running();
    }
}

struct CameraDelegateInner {
    tx: std::sync::mpsc::SyncSender<(i64, Arc<Frame>)>,
    index: std::sync::atomic::AtomicI64,
}

define_obj_type!(
    CameraDelegate + av::capture::VideoDataOutputSampleBufDelegateImpl,
    CameraDelegateInner,
    OTD_CAMERA_DELEGATE
);

impl av::capture::VideoDataOutputSampleBufDelegate for CameraDelegate {}

#[objc::add_methods]
impl av::capture::VideoDataOutputSampleBufDelegateImpl for CameraDelegate {
    extern "C" fn impl_capture_output_did_output_sample_buf_from_connection(
        &mut self,
        _cmd: Option<&cidre::objc::Sel>,
        _output: &av::CaptureOutput,
        sample_buf: &cm::SampleBuf,
        _connection: &av::CaptureConnection,
    ) {
        let Some(pb) = sample_buf.image_buf() else {
            return;
        };
        let Some(frame) = pixel_buf_to_frame(pb) else {
            return;
        };
        let idx = self
            .inner()
            .index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _ = self.inner().tx.try_send((idx, Arc::new(frame)));
    }
}

fn pick_device(spec: &str) -> Result<arc::R<av::CaptureDevice>, String> {
    use av::capture::{DevicePos, DeviceType, DiscoverySession};
    let mut type_list: Vec<&av::capture::DeviceType> =
        vec![DeviceType::built_in_wide_angle_camera()];
    for t in unsafe {
        [
            DeviceType::external(),
            DeviceType::desk_view_camera(),
            DeviceType::continuity_camera(),
        ]
    } {
        if let Some(t) = t {
            type_list.push(t);
        }
    }
    let types = ns::Array::from_slice(&type_list);
    let discovery = DiscoverySession::with_device_types_media_and_pos(
        &types,
        Some(av::MediaType::video()),
        DevicePos::Unspecified,
    );
    let devices = discovery.devices();
    if devices.is_empty() {
        return Err("no video capture devices".into());
    }
    let spec = spec.trim();
    if spec.is_empty() || spec == "0" {
        return Ok(devices.first().unwrap().retained());
    }
    if let Ok(idx) = spec.parse::<usize>() {
        return devices
            .get(idx)
            .map(|d| d.retained())
            .map_err(|_| format!("camera index {idx} out of range"));
    }
    for d in devices.iter() {
        let id = d.unique_id().to_string();
        let name = d.localized_name().to_string();
        if id == spec || name == spec || name.contains(spec) {
            return Ok(d.retained());
        }
    }
    Ok(devices.first().unwrap().retained())
}

fn pick_format(
    device: &av::CaptureDevice,
    want_w: u32,
    want_h: u32,
    want_fps: f64,
) -> (arc::R<av::capture::DeviceFormat>, f64) {
    let want_pixels = (want_w as i64) * (want_h as i64);
    let mut best: Option<(arc::R<av::capture::DeviceFormat>, f64, i64)> = None;
    for fmt in device.formats().iter() {
        let dims = fmt.format_desc().dimensions();
        let w = dims.width.max(1) as u32;
        let h = dims.height.max(1) as u32;
        let pixels = (w as i64) * (h as i64);
        let mut rate = want_fps;
        for range in fmt.video_supported_frame_rate_ranges().iter() {
            let max = range.max_frame_rate();
            let min = range.min_frame_rate();
            if want_fps >= min && want_fps <= max {
                rate = want_fps;
                break;
            }
            if (max - want_fps).abs() < (rate - want_fps).abs() {
                rate = max;
            }
        }
        let score = (pixels - want_pixels).abs();
        if best.as_ref().is_none_or(|(_, _, s)| score < *s) {
            best = Some((fmt.retained(), rate, score));
        }
    }
    best.map(|(f, r, _)| (f, r))
        .unwrap_or_else(|| (device.formats().first().unwrap().retained(), want_fps))
}

pub fn spawn_camera_reader(
    device_spec: &str,
    width: u32,
    height: u32,
    fps: f64,
    queue_depth: usize,
    problem: Arc<std::sync::Mutex<Option<String>>>,
) -> Result<(Reader, Info), String> {
    use av::capture::{DeviceInput, Session, VideoDataOutput};
    use std::sync::atomic::AtomicBool;

    let status = av::CaptureDevice::authorization_status_for_media_type(av::MediaType::video())
        .map_err(ns_error)?;
    if status == av::AuthorizationStatus::Denied || status == av::AuthorizationStatus::Restricted {
        return Err(
            "camera access denied — enable in System Settings ▸ Privacy & Security ▸ Camera".into(),
        );
    }
    if status == av::AuthorizationStatus::NotDetermined {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut block = cidre::blocks::SendBlock::new1(move |ok: bool| {
            let _ = tx.send(ok);
        });
        av::CaptureDevice::request_access_for_media_type_ch(av::MediaType::video(), &mut block)
            .map_err(ns_error)?;
        if !rx.recv().unwrap_or(false) {
            return Err("camera access was not granted".into());
        }
    }

    let device = pick_device(device_spec)?;
    let (format, chosen_fps) = pick_format(&device, width, height, fps);
    let dims = format.format_desc().dimensions();
    let out_w = dims.width.max(1) as u32;
    let out_h = dims.height.max(1) as u32;

    let mut session = Session::new();
    let input =
        DeviceInput::with_device(&device).map_err(|e| e.localized_description().to_string())?;
    let mut video_output = VideoDataOutput::new();
    video_output.set_always_discard_late_video_frames(true);
    let settings = ns::Dictionary::with_keys_values(
        &[cv::pixel_buffer_keys::pixel_format().as_ns()],
        &[cv::PixelFormat::_32_BGRA.as_ref()],
    );
    video_output
        .set_video_settings(Some(&settings))
        .map_err(ns_error)?;

    {
        let mut dev = device.clone();
        let mut lock = dev.config_lock().map_err(ns_error)?;
        lock.set_active_format(&format);
        let frame_duration = cm::Time::with_secs(1.0 / chosen_fps.max(1e-6), 600);
        lock.set_active_video_min_frame_duration(frame_duration)
            .map_err(ns_error)?;
        lock.set_active_video_max_frame_duration(frame_duration)
            .map_err(ns_error)?;
    }

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, frames) = sync_channel(queue_depth.max(1).min(120));
    let queue = dispatch::Queue::new();
    let delegate = CameraDelegate::with(CameraDelegateInner {
        tx,
        index: std::sync::atomic::AtomicI64::new(0),
    });
    let delegate = delegate.retained();
    video_output.set_sample_buf_delegate(Some(delegate.as_ref()), Some(&queue));

    session.configure(|s| {
        if s.can_add_input(&input) {
            s.add_input(&input);
        }
        if s.can_add_output(&video_output) {
            s.add_output(&video_output);
        }
    });

    session.start_running();
    if !session.is_running() {
        if let Ok(mut slot) = problem.lock() {
            *slot = Some("camera session failed to start".into());
        }
        return Err("camera session failed to start".into());
    }

    let hold = MacCaptureHold {
        session: session.retained(),
        _delegate: delegate,
        _queue: queue.retained(),
    };

    Ok((
        Reader {
            frames,
            stop,
            worker: None,
            child: None,
            file_backend: Some(super::MovieDecodeBackend::AvFoundation),
            capture_hold: Some(hold),
            file_decode_hold: None,
        },
        Info {
            width: out_w,
            height: out_h,
            fps: chosen_fps,
            duration: 0.0,
        },
    ))
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetBaseAddress(pixel_buffer: *const cv::PixelBuf) -> *mut std::ffi::c_void;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: *const cv::PixelBuf) -> usize;
}

#[cfg(test)]
mod tests {
    use super::{decode_file_loop, probe, spawn_file_reader};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    fn plasma_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/media/plasma.mp4")
    }

    #[test]
    fn avfoundation_probes_plasma() {
        let path = plasma_path();
        if !path.is_file() {
            return;
        }
        let info = probe(path.to_str().unwrap()).expect("probe");
        assert!(info.width > 0 && info.height > 0);
        assert!(info.fps > 0.0);
    }

    #[test]
    fn avfoundation_decodes_one_frame() {
        let path = plasma_path();
        if !path.is_file() {
            return;
        }
        let path = path.to_str().unwrap().to_string();
        let info = probe(&path).expect("probe");
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = sync_channel(4);
        let worker_stop = stop.clone();
        let worker_path = path.clone();
        let worker_fps = info.fps;
        let reader_slot = Arc::new(std::sync::Mutex::new(None));
        let decode_io = Arc::new(std::sync::Mutex::new(()));
        std::thread::spawn(move || {
            let _ = decode_file_loop(
                &worker_path,
                worker_fps,
                0,
                &tx,
                &worker_stop,
                reader_slot,
                decode_io,
            );
        });
        let (_idx, frame) = rx.recv_timeout(Duration::from_secs(30)).expect("frame");
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(frame.width, info.width);
        assert_eq!(frame.height, info.height);
        assert_eq!(frame.pixels.len(), (info.width * info.height * 4) as usize);
    }

    #[test]
    fn avfoundation_reader_thread_starts() {
        let path = plasma_path();
        if !path.is_file() {
            return;
        }
        let path = path.to_str().unwrap();
        let info = probe(path).expect("probe");
        let problem = Arc::new(std::sync::Mutex::new(None));
        let reader = spawn_file_reader(path, &info, 0, 8, problem.clone()).expect("spawn");
        drop(reader);
        assert!(problem.lock().unwrap().is_none());
    }
}
