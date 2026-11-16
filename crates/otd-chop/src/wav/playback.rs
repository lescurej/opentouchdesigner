use super::types::Wav;

impl Wav {
    pub fn duration(&self) -> f64 {
        let frames = self.samples.len() / self.channels.max(1);
        frames as f64 / self.sample_rate.max(1.0)
    }

    pub fn frame(&self, index: i64) -> Option<&[f32]> {
        let frames = self.samples.len() / self.channels.max(1);
        if frames == 0 {
            return None;
        }
        let index = index.rem_euclid(frames as i64) as usize;
        let at = index * self.channels;
        self.samples.get(at..at + self.channels)
    }

    /// One multi-channel sample at a fractional **audio** frame index (linear interp).
    pub fn sample_at_frame(&self, frame_f: f64) -> Option<Vec<f32>> {
        let frames = self.samples.len() / self.channels.max(1);
        if frames == 0 {
            return None;
        }
        let last = frames.saturating_sub(1) as f64;
        if frame_f <= 0.0 {
            return self.frame(0).map(|s| s.to_vec());
        }
        if frame_f >= last {
            return self.frame(last as i64).map(|s| s.to_vec());
        }
        let i0 = frame_f.floor() as usize;
        let frac = (frame_f - i0 as f64) as f32;
        let i1 = (i0 + 1).min(frames - 1);
        let s0 = self.frame(i0 as i64)?;
        let s1 = self.frame(i1 as i64)?;
        Some(
            s0.iter()
                .zip(s1.iter())
                .map(|(&a, &b)| a + (b - a) * frac)
                .collect(),
        )
    }

    /// Fill `n` output samples by linearly mapping `[src_start, src_end]` in source frames.
    pub fn synthesize_block(
        &self,
        n: usize,
        src_start: f64,
        src_end: f64,
        volume: f32,
    ) -> Vec<Vec<f32>> {
        let chs = self.channels.max(1);
        let mut out: Vec<Vec<f32>> = (0..chs).map(|_| Vec::with_capacity(n)).collect();
        if n == 0 {
            return out;
        }
        let span = src_end - src_start;
        if span.abs() < 1e-12 {
            let frame = src_end.max(0.0);
            if let Some(s) = self.sample_at_frame(frame) {
                for (ch, &v) in out.iter_mut().zip(&s) {
                    ch.resize(n, v * volume);
                }
            }
            return out;
        }
        for i in 0..n {
            let t = (i as f64 + 0.5) / n as f64;
            let frame_f = src_start + t * span;
            if let Some(s) = self.sample_at_frame(frame_f) {
                for (ch, &v) in out.iter_mut().zip(&s) {
                    ch.push(v * volume);
                }
            } else {
                for ch in &mut out {
                    ch.push(0.0);
                }
            }
        }
        out
    }
}
