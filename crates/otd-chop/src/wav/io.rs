use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::path::PathBuf;

use super::types::Wav;

fn u16le(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*b.get(at)?, *b.get(at + 1)?]))
}

fn u32le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *b.get(at)?,
        *b.get(at + 1)?,
        *b.get(at + 2)?,
        *b.get(at + 3)?,
    ]))
}

/// An open 32-bit float WAV. The header is rewritten as samples are appended,
/// so the file stays readable if recording stops without a separate close.
pub struct WavOut {
    path: PathBuf,
    file: File,
    channels: u16,
    frames: u64,
}

impl WavOut {
    pub fn create(path: &Path, channels: usize, sample_rate: f64) -> Result<Self, String> {
        if !(1..=64).contains(&channels) {
            return Err("audio file needs between 1 and 64 channels".into());
        }
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
        let mut file = File::create(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let channels = channels as u16;
        let rate = sample_rate.round().clamp(1.0, 384_000.0) as u32;
        write_header(&mut file, channels, rate, 0)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            file,
            channels,
            frames: 0,
        })
    }

    /// `interleaved` is channel-major within each frame: left, right, left, right.
    pub fn write_frames(&mut self, interleaved: &[f32]) -> Result<(), String> {
        let ch = self.channels as usize;
        if ch == 0 || interleaved.is_empty() {
            return Ok(());
        }
        let frames = interleaved.len() / ch;
        if frames == 0 {
            return Ok(());
        }
        let samples = &interleaved[..frames * ch];
        let new_frames = self.frames.saturating_add(frames as u64);
        let data_bytes = new_frames
            .saturating_mul(self.channels as u64)
            .saturating_mul(4);
        if data_bytes > u32::MAX as u64 {
            return Err(format!(
                "{}: recording is larger than a WAV file can hold",
                self.path.display()
            ));
        }
        for sample in samples {
            self.file
                .write_all(&sample.to_le_bytes())
                .map_err(|e| format!("{}: {e}", self.path.display()))?;
        }
        self.frames = new_frames;
        self.patch_sizes()
    }

    pub fn frames(&self) -> u64 {
        self.frames
    }

    fn patch_sizes(&mut self) -> Result<(), String> {
        let data_bytes = (self.frames * self.channels as u64 * 4) as u32;
        let path = self.path.display();
        self.file
            .seek(SeekFrom::Start(4))
            .map_err(|e| format!("{path}: {e}"))?;
        self.file
            .write_all(&(36 + data_bytes).to_le_bytes())
            .map_err(|e| format!("{path}: {e}"))?;
        self.file
            .seek(SeekFrom::Start(40))
            .map_err(|e| format!("{path}: {e}"))?;
        self.file
            .write_all(&data_bytes.to_le_bytes())
            .map_err(|e| format!("{path}: {e}"))?;
        self.file
            .seek(SeekFrom::End(0))
            .map_err(|e| format!("{path}: {e}"))?;
        self.file.flush().map_err(|e| format!("{path}: {e}"))
    }
}

/// Write one complete float WAV. Used when the input is a whole buffer
/// rather than a stream of new samples.
pub fn write_file(
    path: &Path,
    sample_rate: f64,
    channels: usize,
    interleaved: &[f32],
) -> Result<(), String> {
    let mut out = WavOut::create(path, channels, sample_rate)?;
    out.write_frames(interleaved)
}

fn write_header(file: &mut File, channels: u16, rate: u32, data_bytes: u32) -> std::io::Result<()> {
    let block = channels as u32 * 4;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_bytes).to_le_bytes())?;
    file.write_all(b"WAVE")?;
    file.write_all(b"fmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&3u16.to_le_bytes())?;
    file.write_all(&channels.to_le_bytes())?;
    file.write_all(&rate.to_le_bytes())?;
    file.write_all(&(rate * block).to_le_bytes())?;
    file.write_all(&(block as u16).to_le_bytes())?;
    file.write_all(&32u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&data_bytes.to_le_bytes())?;
    Ok(())
}

pub fn parse(bytes: &[u8]) -> Result<Wav, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }

    let mut format: Option<(u16, u16, u32, u16)> = None;
    let mut data: Option<&[u8]> = None;

    let mut at = 12;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32le(bytes, at + 4).ok_or("truncated chunk header")? as usize;
        let body = bytes
            .get(at + 8..at + 8 + len)
            .ok_or_else(|| format!("truncated `{}` chunk", String::from_utf8_lossy(id)))?;
        match id {
            b"fmt " => {
                let mut tag = u16le(body, 0).ok_or("short fmt chunk")?;
                let channels = u16le(body, 2).ok_or("short fmt chunk")?;
                let rate = u32le(body, 4).ok_or("short fmt chunk")?;
                let bits = u16le(body, 14).ok_or("short fmt chunk")?;
                if tag == 0xfffe {
                    tag = u16le(body, 24).ok_or("short extensible fmt chunk")?;
                }
                format = Some((tag, channels, rate, bits));
            }
            b"data" => data = Some(body),
            _ => {}
        }
        at += 8 + len + (len & 1);
    }

    let (tag, channels, rate, bits) = format.ok_or("no fmt chunk")?;
    let data = data.ok_or("no data chunk")?;
    let channels = channels.max(1) as usize;

    let frames: Vec<f32> = match (tag, bits) {
        (1, 8) => data.iter().map(|b| (*b as f32 - 128.0) / 128.0).collect(),
        (1, 16) => data
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
            .collect(),
        (1, 24) => data
            .chunks_exact(3)
            .map(|c| {
                let v = i32::from_le_bytes([0, c[0], c[1], c[2]]) >> 8;
                v as f32 / 8_388_608.0
            })
            .collect(),
        (1, 32) => data
            .chunks_exact(4)
            .map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f32 / 2_147_483_648.0)
            .collect(),
        (3, 32) => data
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        _ => {
            return Err(format!(
                "unsupported WAV format (tag {tag}, {bits}-bit) — PCM or 32-bit float only"
            ));
        }
    };

    let frame_count = frames.len() / channels;
    Ok(Wav {
        sample_rate: rate as f64,
        channels,
        samples: frames[..frame_count * channels].to_vec(),
    })
}
