use super::Patch;
use otd_core::NodeId;

pub fn rms_channel(p: &Patch, node: NodeId, ch: &str) -> f32 {
    let data = p.data(node);
    let samples = data
        .channel(ch)
        .map(|c| c.samples.as_slice())
        .unwrap_or(&[]);
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

pub fn run_to(p: &mut Patch, id: NodeId, seconds: f64) {
    p.run(id, (seconds * 60.0).round() as usize);
}

pub fn mean_period(samples: &[f32]) -> f32 {
    let mut last = None;
    let mut gaps = Vec::new();
    for (i, w) in samples.windows(2).enumerate() {
        if w[0] < 0.0 && w[1] >= 0.0 {
            if let Some(prev) = last {
                gaps.push((i - prev) as f32);
            }
            last = Some(i);
        }
    }
    gaps.iter().sum::<f32>() / gaps.len().max(1) as f32
}

pub fn zero_crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|w| w[0].signum() != w[1].signum() && w[0] != 0.0 && w[1] != 0.0)
        .count()
}
