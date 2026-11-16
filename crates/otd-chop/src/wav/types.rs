pub const DECODED_RATE: f64 = 48_000.0;

#[derive(Clone, Debug)]
pub struct Wav {
    pub sample_rate: f64,
    pub channels: usize,
    pub samples: Vec<f32>,
}

/// A file turned into PCM. `decoded_at` is 0 when the buffer kept the file's
/// own rate (a PCM WAV). It is the rate ffmpeg was asked for otherwise.
pub struct PreparedWav {
    pub wav: Wav,
    pub decoded_at: f64,
}
