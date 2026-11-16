use crate::data::{Channel, ChopData};

/// Deinterleaves what the worker tapped since the last frame into the CHOP
/// the frame cook shows for the tapped node.
pub fn chop_from_interleaved(samples: &[f32], channels: usize, sample_rate: f64) -> ChopData {
    let ch = channels.max(1);
    let frames = samples.len() / ch;
    let channels = (0..ch)
        .map(|c| {
            let data = (0..frames).map(|fi| samples[fi * ch + c]).collect();
            Channel::new(format!("chan{}", c + 1), data)
        })
        .collect();
    ChopData::new(channels, sample_rate, true)
}
