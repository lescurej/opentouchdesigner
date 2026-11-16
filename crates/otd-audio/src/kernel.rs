pub struct BlockCtx {
    pub sample_rate: f64,
    pub block_len: usize,
}

/// One node of an [`AudioPlan`](crate::AudioPlan). `prepare` runs on the
/// control thread before the plan is published; `process` runs on the audio
/// worker and receives interleaved buffers sized `block_len * channels`.
pub trait AudioKernel: Send {
    fn prepare(&mut self, sample_rate: f64, max_block: usize);
    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]);
}
