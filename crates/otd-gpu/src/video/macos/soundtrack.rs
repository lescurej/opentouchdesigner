use std::path::Path;

use cidre::{av, cf, cm, ns, cat};

use super::track::load_track;
use super::util::ns_error;

/// Read the movie's audio track on the asset timeline and publish it as it arrives.
///
/// Sample 0 is media time 0, the same clock the picture playhead uses.
/// 48 kHz interleaved float, native channel count (stereo, 5.1, 7.1, …).
/// The first buffer is published immediately; later buffers append to it.
pub(crate) fn read_soundtrack(
    path: &str,
    slot: &std::sync::Mutex<otd_core::MovieAudio>,
) -> Result<(), String> {
    const RATE: f64 = 48_000.0;
    let url = cf::Url::with_file_path(Path::new(path)).ok_or("invalid file path")?;
    let asset = av::UrlAsset::with_url(url.as_ns(), None).ok_or("could not open media file")?;
    let Some(track) = load_track(&asset, av::MediaType::audio()) else {
        return Err("no audio".into());
    };

    let format_id = ns::Number::tagged_u32(cat::audio::Format::LINEAR_PCM.0);
    let rate = ns::Number::with_f64(RATE);
    let depth = ns::Number::tagged_i32(32);
    let is_float = ns::Number::with_bool(true);
    let big_endian = ns::Number::with_bool(false);
    let interleaved = ns::Number::with_bool(false);
    let settings = ns::Dictionary::with_keys_values(
        &[
            av::audio::all_formats_keys::id(),
            av::audio::all_formats_keys::sample_rate(),
            av::audio::linear_pcm_keys::bit_depth(),
            av::audio::linear_pcm_keys::is_float(),
            av::audio::linear_pcm_keys::is_big_endian(),
            av::audio::linear_pcm_keys::is_non_interleaved(),
        ],
        &[
            AsRef::<ns::Id>::as_ref(format_id),
            AsRef::<ns::Id>::as_ref(&*rate),
            AsRef::<ns::Id>::as_ref(depth),
            AsRef::<ns::Id>::as_ref(&*is_float),
            AsRef::<ns::Id>::as_ref(&*big_endian),
            AsRef::<ns::Id>::as_ref(&*interleaved),
        ],
    );

    let mut reader = av::asset::Reader::with_asset(&asset).map_err(ns_error)?;
    let mut out =
        av::asset::ReaderTrackOutput::with_track(&track, Some(&settings)).map_err(ns_error)?;
    out.set_always_copies_sample_data(true);
    reader.add_output(&out).map_err(ns_error)?;
    reader.start_reading().map_err(ns_error)?;

    let mut published: Option<std::sync::Arc<otd_core::MoviePcm>> = None;
    let mut filled = 0usize;
    loop {
        let sample = match out.next_sample_buf() {
            Ok(Some(sample)) => sample,
            Ok(None) => break,
            Err(e) => {
                if published.is_some() {
                    break;
                }
                return Err(ns_error(e));
            }
        };
        if let Some((channels, chunk)) = take_pcm(&sample, RATE, filled)? {
            filled += chunk.len() / channels.max(1);
            if let Some(pcm) = &published {
                pcm.push(&chunk);
            } else if !chunk.is_empty() {
                let pcm = std::sync::Arc::new(otd_core::MoviePcm::new(RATE, channels, chunk));
                *slot.lock().unwrap_or_else(|e| e.into_inner()) =
                    otd_core::MovieAudio::Ready(pcm.clone());
                published = Some(pcm);
            }
        }
        if reader.status() == av::asset::ReaderStatus::Completed {
            break;
        }
        if reader.status() == av::asset::ReaderStatus::Failed {
            if published.is_some() {
                break;
            }
            return Err(reader
                .error()
                .map(|e| e.localized_description().to_string())
                .unwrap_or_else(|| "audio reader failed".into()));
        }
    }
    if published.is_none() {
        return Err("no audio".into());
    }
    Ok(())
}

/// One sample buffer as interleaved frames, with silence inserted so the
/// first sample stays at media time 0.
fn take_pcm(
    sample: &cm::SampleBuf,
    rate: f64,
    filled_frames: usize,
) -> Result<Option<(usize, Vec<f32>)>, String> {
    let frames = sample.num_samples().max(0) as usize;
    let Some(block) = sample.data_buf() else {
        return Ok(None);
    };
    let bytes = block.as_slice().map_err(|e| format!("audio buffer: {e}"))?;
    if bytes.len() < 4 || frames == 0 || bytes.len() % 4 != 0 {
        return Err("audio track is not interleaved float PCM".into());
    }
    let n = bytes.len() / 4;
    let ch = (n / frames).max(1);
    if n != frames * ch {
        return Err("audio track is not interleaved float PCM".into());
    }
    let mut pcm = Vec::<f32>::new();
    let pts = sample.pts().as_secs();
    if pts.is_finite() && pts > 0.0 {
        let want = (pts * rate).round() as usize;
        if filled_frames < want {
            pcm.resize((want - filled_frames) * ch, 0.0);
        }
    }
    let data = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const f32, n) };
    pcm.extend_from_slice(data);
    Ok(Some((ch, pcm)))
}