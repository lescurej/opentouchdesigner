use crate::kernel::{AudioKernel, BlockCtx};

pub const MAX_STEP_INPUTS: usize = 16;

#[derive(Clone, Debug)]
pub struct PlanStep {
    pub kernel: usize,
    pub inputs: Vec<usize>,
    pub output: usize,
}

/// A compiled audio graph. Every buffer is allocated by [`AudioPlan::prepare`]
/// on the control thread, so swapping a plan into the worker is a pointer move.
pub struct AudioPlan {
    pub sample_rate: f64,
    pub max_block: usize,
    pub slot_channels: Vec<usize>,
    pub sink_slot: Option<usize>,
    pub tap_slot: Option<usize>,
    pub kernels: Vec<Box<dyn AudioKernel>>,
    pub steps: Vec<PlanStep>,
    bufs: Vec<Vec<f32>>,
}

impl std::fmt::Debug for AudioPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioPlan")
            .field("sample_rate", &self.sample_rate)
            .field("max_block", &self.max_block)
            .field("slot_channels", &self.slot_channels)
            .field("sink_slot", &self.sink_slot)
            .field("tap_slot", &self.tap_slot)
            .field("kernels", &self.kernels.len())
            .field("steps", &self.steps)
            .finish()
    }
}

impl AudioPlan {
    pub fn new(
        max_block: usize,
        slot_channels: Vec<usize>,
        kernels: Vec<Box<dyn AudioKernel>>,
        steps: Vec<PlanStep>,
    ) -> Self {
        Self {
            sample_rate: 0.0,
            max_block: max_block.max(1),
            slot_channels,
            sink_slot: None,
            tap_slot: None,
            kernels,
            steps,
            bufs: Vec::new(),
        }
    }

    pub fn empty(max_block: usize) -> Self {
        Self::new(max_block, Vec::new(), Vec::new(), Vec::new())
    }

    pub fn prepare(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate.max(1.0);
        for step in &mut self.steps {
            step.inputs.truncate(MAX_STEP_INPUTS);
        }
        let block = self.max_block;
        self.bufs = self
            .slot_channels
            .iter()
            .map(|ch| vec![0.0; block * ch.max(&1)])
            .collect();
        for k in &mut self.kernels {
            k.prepare(self.sample_rate, block);
        }
    }

    pub fn channels(&self, slot: Option<usize>) -> usize {
        slot.and_then(|s| self.slot_channels.get(s).copied())
            .unwrap_or(0)
    }

    pub fn sink_channels(&self) -> usize {
        self.channels(self.sink_slot)
    }

    pub fn tap_channels(&self) -> usize {
        self.channels(self.tap_slot)
    }

    pub(crate) fn is_prepared(&self) -> bool {
        self.bufs.len() == self.slot_channels.len()
    }

    pub(crate) fn run_block(&mut self, frames: usize) {
        let ctx = BlockCtx {
            sample_rate: self.sample_rate,
            block_len: frames,
        };
        let Self {
            kernels,
            steps,
            bufs,
            slot_channels,
            ..
        } = self;
        for step in steps.iter() {
            let Some(kernel) = kernels.get_mut(step.kernel) else {
                continue;
            };
            let Some(mut out) = bufs.get_mut(step.output).map(std::mem::take) else {
                continue;
            };
            let mut ins: [&[f32]; MAX_STEP_INPUTS] = [&[]; MAX_STEP_INPUTS];
            let mut n = 0;
            for &slot in &step.inputs {
                if let Some(buf) = bufs.get(slot) {
                    let len = (frames * slot_channels[slot].max(1)).min(buf.len());
                    ins[n] = &buf[..len];
                    n += 1;
                }
            }
            let out_len = (frames * slot_channels[step.output].max(1)).min(out.len());
            kernel.process(&ctx, &ins[..n], &mut out[..out_len]);
            bufs[step.output] = out;
        }
    }

    pub(crate) fn slot(&self, slot: usize, frames: usize) -> &[f32] {
        let Some(buf) = self.bufs.get(slot) else {
            return &[];
        };
        &buf[..(frames * self.slot_channels[slot].max(1)).min(buf.len())]
    }
}
