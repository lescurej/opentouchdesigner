use std::sync::atomic::{AtomicU32, Ordering};

/// A parameter written by the control thread and read once per block by the
/// audio worker, ramped linearly across the block to avoid zipper noise.
pub struct ControlPort {
    target: AtomicU32,
    current: AtomicU32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ramp {
    pub start: f32,
    pub step: f32,
}

impl Ramp {
    pub fn at(&self, i: usize) -> f32 {
        self.start + self.step * (i + 1) as f32
    }
}

impl ControlPort {
    pub fn new(value: f32) -> Self {
        let bits = value.to_bits();
        Self {
            target: AtomicU32::new(bits),
            current: AtomicU32::new(bits),
        }
    }

    pub fn set(&self, value: f32) {
        if value.is_finite() {
            self.target.store(value.to_bits(), Ordering::Relaxed);
        }
    }

    pub fn target(&self) -> f32 {
        f32::from_bits(self.target.load(Ordering::Relaxed))
    }

    pub fn ramp(&self, block_len: usize) -> Ramp {
        let cur = f32::from_bits(self.current.load(Ordering::Relaxed));
        let tgt = self.target();
        if block_len == 0 {
            return Ramp { start: cur, step: 0.0 };
        }
        self.current.store(tgt.to_bits(), Ordering::Relaxed);
        Ramp {
            start: cur,
            step: (tgt - cur) / block_len as f32,
        }
    }
}
