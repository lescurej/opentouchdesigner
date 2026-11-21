use std::sync::{Arc, Mutex};

use otd_audio::{AudioKernel, BlockCtx};

use crate::reastream::{MAX_CHANNELS, ReaStreamLink};

pub const MAX_REASTREAM_CHANNELS: usize = MAX_CHANNELS;

#[derive(Default)]
pub struct ReaperStreamSlot {
    link: Option<ReaStreamLink>,
    active: bool,
}

/// Frame-thread side of one Reaper Out on the worker: reopens the socket
/// only when the address, identifier or rate changes, so the worker's
/// `try_lock` almost never meets a held lock.
pub struct ReaperFeed {
    slot: Arc<Mutex<ReaperStreamSlot>>,
    key: Option<String>,
    active: bool,
    error: Option<String>,
}

impl ReaperFeed {
    pub fn new() -> (Self, Arc<Mutex<ReaperStreamSlot>>) {
        let slot = Arc::new(Mutex::new(ReaperStreamSlot::default()));
        (
            Self {
                slot: slot.clone(),
                key: None,
                active: false,
                error: None,
            },
            slot,
        )
    }

    pub fn sync(&mut self, host: &str, identifier: &str, rate: u32, active: bool) -> Option<&str> {
        let key = format!("{host}|{identifier}|{rate}");
        if self.key.as_deref() == Some(key.as_str()) && self.active == active {
            return self.error.as_deref();
        }
        let link = match ReaStreamLink::open(host, identifier, rate) {
            Ok(link) => {
                self.error = None;
                Some(link)
            }
            Err(e) => {
                log::warn!("ReaStream: {e}");
                self.error = Some(e);
                None
            }
        };
        if let Ok(mut slot) = self.slot.lock() {
            slot.link = link;
            slot.active = active;
        }
        self.key = Some(key);
        self.active = active;
        self.error.as_deref()
    }
}

/// Passes audio through and sends each rendered block to REAPER on the spot,
/// so the stream runs at exactly the worker's sample clock.
pub struct ReaperSinkKernel {
    slot: Arc<Mutex<ReaperStreamSlot>>,
    channels: usize,
    planar: Vec<Vec<f32>>,
    packet: Vec<u8>,
}

impl ReaperSinkKernel {
    pub fn new(slot: Arc<Mutex<ReaperStreamSlot>>, channels: usize) -> Self {
        Self {
            slot,
            channels: channels.clamp(1, MAX_REASTREAM_CHANNELS),
            planar: Vec::new(),
            packet: Vec::new(),
        }
    }
}

impl AudioKernel for ReaperSinkKernel {
    fn prepare(&mut self, _: f64, max_block: usize) {
        self.planar = (0..self.channels).map(|_| vec![0.0; max_block]).collect();
        self.packet = Vec::with_capacity(crate::reastream::MAX_AUDIO_BYTES + 64);
    }

    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]) {
        let input = inputs.first().copied().unwrap_or(&[]);
        for (i, s) in output.iter_mut().enumerate() {
            *s = input.get(i).copied().unwrap_or(0.0);
        }
        let Ok(guard) = self.slot.try_lock() else {
            return;
        };
        let Some(link) = guard.link.as_ref().filter(|_| guard.active) else {
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
        link.send(&refs[..ch], &mut self.packet);
    }
}
