use super::super::*;
use crate::ops::ChopCtx;

pub(crate) struct Spectrum {
    windows: Vec<Vec<f32>>,
    fft_size: usize,
    fft: Option<std::sync::Arc<dyn rustfft::Fft<f32>>>,
    planner: rustfft::FftPlanner<f32>,
}

pub(crate) fn params_spectrum() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "mode".into(),
        Param::menu("timetoraw", &["timetoraw", "visual"]).with_label("Mode"),
    );
    m.insert(
        "size".into(),
        Param::menu("1024", &["256", "512", "1024", "2048", "4096"]).with_label("FFT Size"),
    );
    m.insert(
        "bands".into(),
        Param::int(4).with_label("Bands").with_range(1.0, 32.0),
    );
    m.insert(
        "gain".into(),
        Param::float(1.0).with_label("Gain").with_range(0.0, 64.0),
    );
    m.insert("decibels".into(), Param::bool(false).with_label("Decibels"));
    m
}

fn spectrum_push_window(windows: &mut [Vec<f32>], input: &ChopData, size: usize) {
    for (i, w) in windows.iter_mut().enumerate() {
        if let Some(ch) = input.channels.get(i) {
            w.extend_from_slice(&ch.samples);
        }
        if w.len() > size {
            let excess = w.len() - size;
            w.drain(..excess);
        }
    }
}

fn spectrum_fft_window(
    samples: &[f32],
    size: usize,
    fft: &dyn rustfft::Fft<f32>,
) -> Vec<rustfft::num_complex::Complex<f32>> {
    let mut buffer: Vec<rustfft::num_complex::Complex<f32>> = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (size - 1).max(1) as f32).cos();
            rustfft::num_complex::Complex::new(s * w, 0.0)
        })
        .collect();
    fft.process(&mut buffer);
    buffer
}

fn spectrum_mag_phase_channels(
    buffer: &[rustfft::num_complex::Complex<f32>],
    size: usize,
    gain: f32,
    decibels: bool,
    base_name: &str,
) -> (Channel, Channel) {
    let half = size / 2;
    let scale = 2.0 / size as f32;
    let deg = 180.0 / std::f32::consts::PI;
    let mut mag = Vec::with_capacity(half);
    let mut phase = Vec::with_capacity(half);
    for (k, c) in buffer.iter().enumerate().take(half) {
        let mut m = if k == 0 {
            c.norm() / size as f32
        } else {
            c.norm() * scale
        };
        m *= gain;
        if decibels {
            m = 20.0 * m.max(1e-12).log10();
        }
        mag.push(m);
        phase.push(c.arg() * deg);
    }
    (
        Channel::new(format!("{base_name}_m"), mag),
        Channel::new(format!("{base_name}_p"), phase),
    )
}

pub(crate) fn cook_spectrum_visual(
    magnitudes: &[f32],
    bands: usize,
    gain: f32,
    decibels: bool,
    rate: f64,
    c: &ChopCtx<'_>,
) -> ChopData {
    let n = slice_len(rate, c.time).min(64);
    let half = magnitudes.len();
    let channels = (0..bands)
        .map(|b| {
            let lo_f = (b as f32 / bands as f32).powf(3.0);
            let hi_f = ((b + 1) as f32 / bands as f32).powf(3.0);
            let lo = (lo_f * half as f32) as usize;
            let hi = ((hi_f * half as f32) as usize).max(lo + 1).min(half);
            let energy: f32 = magnitudes[lo..hi].iter().sum::<f32>() / (hi - lo).max(1) as f32;
            let mut v = energy * gain;
            if decibels {
                v = 20.0 * (v.max(1e-6)).log10();
            }
            Channel::constant(format!("band{}", b + 1), v, n)
        })
        .collect();
    ChopData::new(channels, c.rate(), true)
}

pub(crate) fn cook_spectrum(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let input_rate = input.sample_rate.max(c.rate());
    let visual = c.menu("mode") == 1;
    let size: usize = c
        .node
        .param("size")
        .map(|p| p.eval(c.eval).as_str())
        .and_then(|s| s.parse().ok())
        .unwrap_or(1024);
    let bands = c
        .node
        .param("bands")
        .map(|p| p.eval(c.eval).as_i64())
        .unwrap_or(4)
        .clamp(1, 32) as usize;
    let gain = c
        .node
        .param("gain")
        .map(|p| p.eval(c.eval).as_f32())
        .unwrap_or(1.0);
    let decibels = c
        .node
        .param("decibels")
        .map(|p| p.eval(c.eval).as_bool())
        .unwrap_or(false);

    let names: Vec<String> = if input.channels.is_empty() {
        vec!["chan1".into()]
    } else {
        input.channels.iter().map(|ch| ch.name.clone()).collect()
    };
    let ch_count = names.len();

    let path = c.path.to_string();
    let entry = c.io.spectra.entry(path).or_insert_with(|| Spectrum {
        windows: Vec::new(),
        fft_size: 0,
        fft: None,
        planner: rustfft::FftPlanner::new(),
    });

    if entry.fft_size != size {
        entry.fft_size = size;
        entry.fft = Some(entry.planner.plan_fft_forward(size));
        entry.windows.clear();
    }
    if entry.windows.len() != ch_count {
        entry.windows = vec![Vec::new(); ch_count];
    }

    spectrum_push_window(&mut entry.windows, &input, size);

    let half = size / 2;
    let window_ready = entry.windows.first().is_some_and(|w| w.len() >= size);
    let Some(fft) = entry.fft.clone() else {
        if visual {
            let n = slice_len(input_rate, c.time).min(64);
            let channels = (0..bands)
                .map(|i| Channel::constant(format!("band{}", i + 1), 0.0, n))
                .collect();
            return ChopData::new(channels, c.rate(), true);
        }
        let mut channels = Vec::new();
        for name in &names {
            channels.push(Channel::new(format!("{name}_m"), vec![0.0; half]));
            channels.push(Channel::new(format!("{name}_p"), vec![0.0; half]));
        }
        return ChopData::new(channels, input_rate, false);
    };

    if !window_ready {
        if visual {
            let n = slice_len(input_rate, c.time).min(64);
            let channels = (0..bands)
                .map(|i| Channel::constant(format!("band{}", i + 1), 0.0, n))
                .collect();
            return ChopData::new(channels, c.rate(), true);
        }
        let mut channels = Vec::new();
        for name in &names {
            channels.push(Channel::new(format!("{name}_m"), vec![0.0; half]));
            channels.push(Channel::new(format!("{name}_p"), vec![0.0; half]));
        }
        return ChopData::new(channels, input_rate, false);
    }

    if visual {
        let buffer = spectrum_fft_window(&entry.windows[0], size, fft.as_ref());
        let half = size / 2;
        let magnitudes: Vec<f32> = buffer[..half]
            .iter()
            .enumerate()
            .map(|(k, c)| {
                if k == 0 {
                    c.norm() / size as f32
                } else {
                    c.norm() * 2.0 / size as f32
                }
            })
            .collect();
        return cook_spectrum_visual(&magnitudes, bands, gain, decibels, input_rate, c);
    }

    let mut channels = Vec::with_capacity(ch_count * 2);
    for (i, name) in names.iter().enumerate() {
        let buffer = spectrum_fft_window(&entry.windows[i], size, fft.as_ref());
        let (m, p) = spectrum_mag_phase_channels(&buffer, size, gain, decibels, name);
        channels.push(m);
        channels.push(p);
    }
    ChopData::new(channels, input_rate, false)
}
