use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use otd_audio::{AudioKernel, BlockCtx};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::data::ChopData;

const BACKLOG_BLOCKS: usize = 40;
/// Below this a time-sliced source is a control signal: the worker holds its
/// latest value instead of playing its few samples as audio.
const STREAM_MIN_RATE: f64 = 8_000.0;

const HOLD: u8 = 0;
const STREAM: u8 = 1;
const WHOLE: u8 = 2;

struct Shared {
    mode: AtomicU8,
    hold: Vec<AtomicU32>,
    whole: Mutex<Option<Arc<ChopData>>>,
}

/// How a frame-cooked source crosses into the worker, decided per publish:
/// audio-rate slices stream through a FIFO, control slices hold their latest
/// value, and a standalone buffer (a wavetable) is handed over whole.
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

/// Lets a worker kernel read a bridged source's whole buffer.
#[derive(Clone)]
pub struct BridgeLink(Arc<Shared>);

impl BridgeLink {
    pub fn whole(&self) -> Option<Arc<ChopData>> {
        if self.0.mode.load(Ordering::Acquire) != WHOLE {
            return None;
        }
        self.0.whole.lock().ok()?.clone()
    }
}

pub fn bridge(channels: usize, capacity_frames: usize) -> (BridgeFeed, BridgeKernel) {
    let channels = channels.max(1);
    let (producer, consumer) = RingBuffer::new(capacity_frames.max(1) * channels);
    let shared = Arc::new(Shared {
        mode: AtomicU8::new(HOLD),
        hold: (0..channels).map(|_| AtomicU32::new(0)).collect(),
        whole: Mutex::new(None),
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
    pub fn link(&self) -> BridgeLink {
        BridgeLink(self.shared.clone())
    }

    pub fn publish(&mut self, data: &Arc<ChopData>) {
        let src_ch = data.num_channels();
        if src_ch == 0 {
            return;
        }
        let frames = data.num_samples();
        let pick = |c: usize| &data.channels[c.min(src_ch - 1)].samples;
        for (c, hold) in self.shared.hold.iter().enumerate() {
            let v = pick(c).last().copied().unwrap_or(0.0);
            hold.store(v.to_bits(), Ordering::Relaxed);
        }
        if frames <= 1 {
            self.shared.mode.store(HOLD, Ordering::Release);
            return;
        }
        if !data.time_sliced {
            if let Ok(mut whole) = self.shared.whole.lock() {
                *whole = Some(data.clone());
            }
            self.shared.mode.store(WHOLE, Ordering::Release);
            return;
        }
        if data.sample_rate < STREAM_MIN_RATE {
            self.shared.mode.store(HOLD, Ordering::Release);
            return;
        }
        let ch = self.channels;
        let fit = frames.min(self.producer.slots() / ch);
        self.scratch.clear();
        for fi in 0..fit {
            for c in 0..ch {
                self.scratch.push(pick(c).get(fi).copied().unwrap_or(0.0));
            }
        }
        let _ = self.producer.push_partial_slice(&self.scratch);
        self.shared.mode.store(STREAM, Ordering::Release);
    }
}

impl AudioKernel for BridgeKernel {
    fn prepare(&mut self, _: f64, max_block: usize) {
        self.max_backlog = max_block * BACKLOG_BLOCKS * self.channels;
    }

    fn process(&mut self, ctx: &BlockCtx, _: &[&[f32]], output: &mut [f32]) {
        let ch = self.channels;
        if self.shared.mode.load(Ordering::Acquire) != STREAM {
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
