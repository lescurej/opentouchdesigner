use std::sync::atomic::AtomicBool;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};

use av::capture::VideoDataOutputSampleBufDelegate;
use cidre::{arc, av, cm, cv, define_obj_type, dispatch, ns, objc};

use super::pixel::pixel_buf_to_frame;
use super::util::ns_error;
use super::super::{Frame, Info, MovieDecodeBackend, Reader};

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
    }
    .into_iter()
    .flatten()
    {
        type_list.push(t);
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
    problem: Arc<Mutex<Option<String>>>,
) -> Result<(Reader, Info), String> {
    use av::capture::{DeviceInput, Session, VideoDataOutput};

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
    let (tx, frames) = sync_channel(queue_depth.clamp(1, 120));
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
            file_backend: Some(MovieDecodeBackend::AvFoundation),
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