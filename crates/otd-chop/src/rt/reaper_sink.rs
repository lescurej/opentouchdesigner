use std::sync::{Arc, Mutex};

use otd_audio::{AudioKernel, BlockCtx};

use crate::reastream::ReaStreamSender;

pub const MAX_REASTREAM_CHANNELS: usize = 64;

#[derive(Default)]
pub struct ReaperStreamSlot {
    pub sender: Option<ReaStreamSender>,
    pub target: String,
    pub active: bool,
    pub error: Option<String>,
}

/// Passes audio through and streams it to REAPER. The slot is configured by
/// the frame thread; the worker only `try_lock`s it, so a slow reconnect
/// drops a block instead of stalling the worker.
pub struct ReaperSinkKernel {
    slot: Arc<Mutex<ReaperStreamSlot>>,
    channels: usize,
    planar: Vec<Vec<f32>>,
}

impl ReaperSinkKernel {
    pub fn new(slot: Arc<Mutex<ReaperStreamSlot>>, channels: usize) -> Self {
        Self {
            slot,
            channels: channels.clamp(1, MAX_REASTREAM_CHANNELS),
            planar: Vec::new(),
        }
    }
}

impl AudioKernel for ReaperSinkKernel {
    fn prepare(&mut self, _: f64, max_block: usize) {
        self.planar = (0..self.channels).map(|_| vec![0.0; max_block]).collect();
    }

    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]) {
        let input = inputs.first().copied().unwrap_or(&[]);
        for (i, s) in output.iter_mut().enumerate() {
            *s = input.get(i).copied().unwrap_or(0.0);
        }
        let Ok(guard) = self.slot.try_lock() else {
            return;
        };
        let Some(sender) = guard.sender.as_ref().filter(|_| guard.active) else {
            return;
        };
        let ch = self.channels;
        let frames = ctx.block_len.min(output.len() / ch);
        for (c, plane) in self.planar.iter_mut().enumerate() {
            for (fi, s) in plane.iter_mut().take(frames).enumerate() {
                *s = output[fi * ch + c];
            }
        }
        let mut refs: [&[f32]; MAX_REASTREAM_CHANNELS] = [&[]; MAX_REASTREAM_CHANNELS];
        for (r, plane) in refs.iter_mut().zip(&self.planar) {
            *r = &plane[..frames];
        }
        sender.push(&refs[..ch]);
    }
}
