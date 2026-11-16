use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

use otd_audio::{AudioKernel, BlockCtx};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::data::ChopData;

const BACKLOG_BLOCKS: usize = 8;

struct Shared {
    streaming: AtomicBool,
    hold: Vec<AtomicU32>,
}

/// Frame-thread end of a bridge: streams a frame CHOP's time slices into the
/// worker FIFO, or holds a constant when the source is not a stream.
pub struct BridgeFeed {
    producer: Producer<f32>,
    shared: Arc<Shared>,
    channels: usize,
    scratch: Vec<f32>,
}

/// Worker end: pops exactly one block per call and repeats the last frame on
/// underflow so a late frame thread is heard as a hold, not a click.
pub struct BridgeKernel {
    consumer: Consumer<f32>,
    shared: Arc<Shared>,
    channels: usize,
    last: Vec<f32>,
    max_backlog: usize,
}

pub fn bridge(channels: usize, capacity_frames: usize) -> (BridgeFeed, BridgeKernel) {
    let channels = channels.max(1);
    let (producer, consumer) = RingBuffer::new(capacity_frames.max(1) * channels);
    let shared = Arc::new(Shared {
        streaming: AtomicBool::new(false),
        hold: (0..channels).map(|_| AtomicU32::new(0)).collect(),
    });
    (
        BridgeFeed {
            producer,
            shared: shared.clone(),
            channels,
            scratch: Vec::new(),
        },
        BridgeKernel {
            consumer,
            shared,
            channels,
            last: vec![0.0; channels],
            max_backlog: 0,
        },
    )
}

impl BridgeFeed {
    pub fn publish(&mut self, data: &ChopData) {
        let src_ch = data.num_channels();
        if src_ch == 0 {
            return;
        }
        let frames = data.num_samples();
        let ch = self.channels;
        let pick = |c: usize| &data.channels[c.min(src_ch - 1)].samples;
        if !(data.time_sliced && frames > 1) {
            for (c, hold) in self.shared.hold.iter().enumerate() {
                let v = pick(c).last().copied().unwrap_or(0.0);
                hold.store(v.to_bits(), Ordering::Relaxed);
            }
            self.shared.streaming.store(false, Ordering::Release);
            return;
        }
        let fit = frames.min(self.producer.slots() / ch);
        self.scratch.clear();
        for fi in 0..fit {
            for c in 0..ch {
                self.scratch.push(pick(c).get(fi).copied().unwrap_or(0.0));
            }
        }
        let _ = self.producer.push_partial_slice(&self.scratch);
        self.shared.streaming.store(true, Ordering::Release);
    }
}

impl AudioKernel for BridgeKernel {
    fn prepare(&mut self, _: f64, max_block: usize) {
        self.max_backlog = max_block * BACKLOG_BLOCKS * self.channels;
    }

    fn process(&mut self, ctx: &BlockCtx, _: &[&[f32]], output: &mut [f32]) {
        let ch = self.channels;
        if !self.shared.streaming.load(Ordering::Acquire) {
            for (i, s) in output.iter_mut().enumerate() {
                *s = f32::from_bits(self.shared.hold[i % ch].load(Ordering::Relaxed));
            }
            return;
        }
        let queued = self.consumer.slots();
        if queued > self.max_backlog {
            let excess = (queued - self.max_backlog) / ch * ch;
            if let Ok(chunk) = self.consumer.read_chunk(excess) {
                chunk.commit_all();
            }
        }
        let want = (ctx.block_len * ch).min(output.len());
        let (filled, rest) = self.consumer.pop_partial_slice(&mut output[..want]);
        let got = filled.len();
        if got >= ch {
            self.last.copy_from_slice(&filled[got - ch..got]);
        }
        for (i, s) in rest.iter_mut().enumerate() {
            *s = self.last[(got + i) % ch];
        }
    }
}
