use std::sync::Arc;

use otd_audio::{AudioKernel, BlockCtx, ControlPort};
use otd_core::{CookContext, Crossings, EvalContext, NodeId};

use crate::data::{Channel, ChopData};
use crate::engine::ChannelStore;
use crate::io::Io;
use crate::ops::{is_rt_safe_chop, spec_for, ChopCtx, OpState, NULL};

use super::frame_share::FrameShare;

/// Runs a frame CHOP's cook function once per audio block on the worker.
/// Input `ChopData` buffers are reused in place; the cook's own output still
/// allocates, which is why plans run on the worker and not in the device
/// callback.
pub struct CookWrapKernel {
    id: NodeId,
    cook: fn(&mut ChopCtx) -> ChopData,
    state: OpState,
    path: String,
    input_count: usize,
    store: ChannelStore,
    share: Arc<FrameShare>,
    io: Io,
    io_generation: u64,
    inputs: Vec<Arc<ChopData>>,
    clock: Option<CookContext>,
}

const CLOCK_RESYNC_SECONDS: f64 = 0.5;

impl CookWrapKernel {
    pub fn build(graph: &otd_core::Graph, id: NodeId, share: Arc<FrameShare>) -> Option<Self> {
        let node = graph.get(id)?;
        let op = node.op_type.as_str();
        if op == crate::io::AUDIO_OUT || op == crate::io::REAPER_OUT {
            return None;
        }
        if !(op == NULL || is_rt_safe_chop(op)) {
            return None;
        }
        let input_count = node.inputs.len();
        Some(Self {
            id,
            cook: spec_for(op)?.cook,
            state: OpState::default(),
            path: graph.path(id),
            input_count,
            store: ChannelStore::default(),
            share,
            io: Io::default(),
            io_generation: u64::MAX,
            inputs: (0..input_count).map(|_| Arc::new(ChopData::empty())).collect(),
            clock: None,
        })
    }

    /// Advances one block from the previous block, so time keeps moving
    /// between frames; snaps back to the frame clock on a seek or a stall.
    fn advance_clock(&mut self, frames: usize, sample_rate: f64) -> CookContext {
        let frame = self.share.time();
        let dt = frames as f64 / sample_rate.max(1.0);
        let next = match self.clock {
            Some(prev) if (prev.time + dt - frame.time).abs() <= CLOCK_RESYNC_SECONDS => {
                let time = prev.time + dt;
                CookContext {
                    time,
                    abs_time: prev.abs_time + dt,
                    frame: frame.frame + ((time - frame.time) * frame.fps).floor() as i64,
                    ..frame
                }
            }
            _ => frame,
        };
        let next = CookContext {
            dt,
            sample_rate,
            ..next
        };
        self.clock = Some(next);
        next
    }
}

impl AudioKernel for CookWrapKernel {
    fn prepare(&mut self, sample_rate: f64, _: usize) {
        for input in &mut self.inputs {
            *input = Arc::new(ChopData::new(Vec::new(), sample_rate, true));
        }
    }

    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]) {
        let n = ctx.block_len;
        let Some(graph) = self.share.graph() else {
            output.fill(0.0);
            return;
        };
        let Some(node) = graph.get(self.id) else {
            output.fill(0.0);
            return;
        };
        let generation = self.share.io_generation();
        if generation != self.io_generation {
            self.share.refresh_io(&mut self.io);
            self.io_generation = generation;
        }
        for (i, slot) in self.inputs.iter_mut().enumerate().take(self.input_count) {
            let buf = inputs.get(i).copied().unwrap_or(&[]);
            fill_chop(slot, buf, n, ctx.sample_rate);
        }
        let time = self.advance_clock(n, ctx.sample_rate);
        let (t, abs, frame) = graph.local_clock_at(self.id, &time);
        let eval = EvalContext {
            frame,
            time: t,
            abs_time: abs,
            fps: time.fps,
            channels: None,
            path: Some(&self.path),
            locals: &[],
        };
        let mut chop_ctx = ChopCtx {
            graph: &graph,
            id: self.id,
            node,
            eval: &eval,
            time: &time,
            inputs: std::mem::take(&mut self.inputs),
            foreign: Crossings::default(),
            state: &mut self.state,
            io: &mut self.io,
            path: &self.path,
            store: &self.store,
            rate_override: Some(ctx.sample_rate),
        };
        let cooked = (self.cook)(&mut chop_ctx);
        self.inputs = std::mem::take(&mut chop_ctx.inputs);
        write_interleaved(&cooked, output, n);
    }
}

/// Final gain stage of an Audio Device Out; the frame thread drives
/// `volume` through the shared [`ControlPort`].
pub struct SinkGainKernel {
    pub volume: Arc<ControlPort>,
    pub channels: usize,
}

impl AudioKernel for SinkGainKernel {
    fn prepare(&mut self, _: f64, _: usize) {}

    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]) {
        let ch = self.channels.max(1);
        let ramp = self.volume.ramp(ctx.block_len);
        let input = inputs.first().copied().unwrap_or(&[]);
        for (i, s) in output.iter_mut().enumerate() {
            *s = input.get(i).copied().unwrap_or(0.0) * ramp.at(i / ch);
        }
    }
}

fn fill_chop(slot: &mut Arc<ChopData>, buf: &[f32], frames: usize, rate: f64) {
    let ch = buf.len().checked_div(frames).unwrap_or(0).max(1);
    if Arc::get_mut(slot).is_none() {
        *slot = Arc::new(ChopData::new(Vec::new(), rate, true));
    }
    let Some(data) = Arc::get_mut(slot) else {
        return;
    };
    data.sample_rate = rate;
    data.time_sliced = true;
    if buf.is_empty() {
        data.channels.clear();
        return;
    }
    data.channels.truncate(ch);
    while data.channels.len() < ch {
        let name = format!("chan{}", data.channels.len() + 1);
        data.channels.push(Channel::new(name, Vec::with_capacity(frames)));
    }
    for (c, channel) in data.channels.iter_mut().enumerate() {
        channel.samples.resize(frames, 0.0);
        for (fi, s) in channel.samples.iter_mut().enumerate() {
            *s = buf.get(fi * ch + c).copied().unwrap_or(0.0);
        }
    }
}

pub(crate) fn write_interleaved(data: &ChopData, out: &mut [f32], frames: usize) {
    if frames == 0 || data.channels.is_empty() {
        out.fill(0.0);
        return;
    }
    let ch = (out.len() / frames).max(1);
    for fi in 0..frames {
        for c in 0..ch {
            let channel = data.channels.get(c).unwrap_or(&data.channels[0]);
            let s = channel
                .samples
                .get(fi)
                .or_else(|| channel.samples.last())
                .copied()
                .unwrap_or(0.0);
            if let Some(o) = out.get_mut(fi * ch + c) {
                *o = if s.is_finite() { s } else { 0.0 };
            }
        }
    }
}

pub fn can_compile_rt_node(graph: &otd_core::Graph, id: NodeId) -> bool {
    let op = graph.node(id).op_type.as_str();
    if op == crate::io::AUDIO_OUT || op == crate::io::REAPER_OUT {
        return true;
    }
    (op == NULL || is_rt_safe_chop(op)) && spec_for(op).is_some()
}
