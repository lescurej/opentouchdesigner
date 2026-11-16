/// Editor setting: how much audio the output keeps queued.
pub const AUDIO_LATENCY_MIN_MS: u32 = 20;
pub const AUDIO_LATENCY_MAX_MS: u32 = 300;
pub const AUDIO_LATENCY_DEFAULT_MS: u32 = 200;
pub fn clamp_audio_latency_ms(ms: u32) -> u32 {
    ms.clamp(AUDIO_LATENCY_MIN_MS, AUDIO_LATENCY_MAX_MS)
}

/// `(target, cap)` in samples. `ms == 0` is the default, for a cook that
/// never received the editor setting.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn latency_samples(sample_rate: f64, ms: u32) -> (usize, usize) {
    const AUDIO_LATENCY_HEADROOM_MS: u32 = 100;
    let ms = if ms == 0 {
        AUDIO_LATENCY_DEFAULT_MS
    } else {
        clamp_audio_latency_ms(ms)
    };
    let samples = |ms: u32| {
        ((sample_rate.max(1.0) * f64::from(ms)) / 1000.0)
            .round()
            .max(1.0) as usize
    };
    let target = samples(ms);
    let cap = samples(ms.saturating_add(AUDIO_LATENCY_HEADROOM_MS)).max(target);
    (target, cap)
}

/// Rate to open at. The project rate when the device lists it, otherwise
/// 48 kHz, otherwise the top of the device's range.
pub(crate) fn device_rate(min: u32, max: u32, want: u32) -> u32 {
    if min <= max && (min..=max).contains(&want) {
        want
    } else if min <= max && (min..=max).contains(&48_000) {
        48_000
    } else {
        max
    }
}

/// CoreAudio reports one config per discrete rate, and 44.1 kHz is first.
/// Opening that config is what moves the system clock to 44.1 kHz. Pick the
/// span that contains `want` instead of the first one with the right channels.
pub(crate) fn rate_in_spans(spans: &[(u32, u32)], want: u32) -> u32 {
    let contains = |span: &(u32, u32), rate: u32| span.0 <= rate && rate <= span.1;
    if let Some(span) = spans.iter().find(|span| contains(span, want)) {
        return device_rate(span.0, span.1, want);
    }
    if let Some(span) = spans.iter().find(|span| contains(span, 48_000)) {
        return device_rate(span.0, span.1, 48_000);
    }
    spans
        .iter()
        .max_by_key(|span| span.1)
        .map(|span| device_rate(span.0, span.1, want))
        .unwrap_or(want)
}

pub(super) fn open_rate(ranges: &[&cpal::SupportedStreamConfigRange], want: u32) -> u32 {
    let spans: Vec<(u32, u32)> = ranges
        .iter()
        .map(|range| (range.min_sample_rate(), range.max_sample_rate()))
        .collect();
    rate_in_spans(&spans, want)
}

pub(crate) fn rate_mismatch_note(wanted: f64, actual: f64) -> Option<String> {
    if (wanted - actual).abs() < 1.0 {
        None
    } else {
        Some(format!(
            "opened at {actual:.0} Hz — {wanted:.0} Hz is outside this device's range"
        ))
    }
}

pub(super) fn input_stream_config(
    device: &cpal::Device,
    want_rate: u32,
) -> Result<cpal::SupportedStreamConfig, String> {
    use cpal::traits::DeviceTrait;
    let ranges: Vec<_> = device
        .supported_input_configs()
        .map_err(|e| e.to_string())?
        .filter(|range| range.sample_format() == cpal::SampleFormat::F32)
        .collect();
    if ranges.is_empty() {
        return device.default_input_config().map_err(|e| e.to_string());
    }
    let prefer = device
        .default_input_config()
        .ok()
        .map(|config| config.channels());
    let pool: Vec<_> = if let Some(channels) = prefer {
        let exact: Vec<_> = ranges
            .iter()
            .filter(|range| range.channels() == channels)
            .collect();
        if exact.is_empty() {
            let widest = ranges.iter().map(|range| range.channels()).max();
            ranges
                .iter()
                .filter(|range| Some(range.channels()) == widest)
                .collect()
        } else {
            exact
        }
    } else {
        let widest = ranges.iter().map(|range| range.channels()).max();
        ranges
            .iter()
            .filter(|range| Some(range.channels()) == widest)
            .collect()
    };
    let rate = open_rate(&pool, want_rate);
    let range = pool
        .iter()
        .find(|range| range.min_sample_rate() <= rate && rate <= range.max_sample_rate())
        .ok_or_else(|| "no input configuration".to_string())?;
    Ok(range.with_sample_rate(rate))
}

/// Widest f32 output that can carry `want_channels`, opened at the project
/// sample rate when the device allows it. Returns the chosen config and the
/// widest channel count the device offered.
pub(super) fn output_stream_config(
    device: &cpal::Device,
    want_channels: usize,
    want_rate: u32,
) -> Result<(cpal::SupportedStreamConfig, usize), String> {
    use cpal::traits::DeviceTrait;
    let ranges: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| e.to_string())?
        .filter(|range| range.sample_format() == cpal::SampleFormat::F32)
        .collect();
    if ranges.is_empty() {
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        let n = config.channels() as usize;
        return Ok((config, n));
    }
    let max_ch = ranges
        .iter()
        .map(|range| range.channels() as usize)
        .max()
        .unwrap_or(1);
    let target = want_channels.clamp(1, max_ch) as u16;
    let exact: Vec<_> = ranges
        .iter()
        .filter(|range| range.channels() == target)
        .collect();
    let pool: Vec<_> = if !exact.is_empty() {
        exact
    } else {
        let wider: Vec<_> = ranges
            .iter()
            .filter(|range| range.channels() as usize >= want_channels)
            .collect();
        if wider.is_empty() {
            let widest = ranges.iter().map(|range| range.channels()).max();
            ranges
                .iter()
                .filter(|range| Some(range.channels()) == widest)
                .collect()
        } else {
            let narrowest = wider.iter().map(|range| range.channels()).min();
            wider
                .into_iter()
                .filter(|range| Some(range.channels()) == narrowest)
                .collect()
        }
    };
    let rate = open_rate(&pool, want_rate);
    let range = pool
        .iter()
        .find(|range| range.min_sample_rate() <= rate && rate <= range.max_sample_rate())
        .ok_or_else(|| "no output configuration".to_string())?;
    Ok((range.with_sample_rate(rate), max_ch))
}