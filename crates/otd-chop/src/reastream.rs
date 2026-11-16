//! ReaStream UDP audio (`MRSR` packets) for the Reaper ReaStream plugin.

use std::collections::VecDeque;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use otd_core::stop_io;

pub const DEFAULT_PORT: u16 = 58710;
/// Largest audio payload per packet (bytes).
pub const MAX_AUDIO_BYTES: usize = 1200;

const MAGIC: &[u8; 4] = b"MRSR";

/// Build one or more ReaStream audio packets from non-interleaved channel slices
/// (`channels[0]` then `channels[1]`, … per packet).
pub fn encode_audio_packets(
    identifier: &str,
    sample_rate: u32,
    channels: &[&[f32]],
) -> Result<Vec<Vec<u8>>, String> {
    if channels.is_empty() {
        return Ok(Vec::new());
    }
    let nch = channels.len();
    if nch > 64 {
        return Err(format!("ReaStream supports at most 64 channels (got {nch})"));
    }
    if identifier.len() >= 32 {
        return Err("identifier must be shorter than 32 characters".into());
    }
    let n = channels.iter().map(|c| c.len()).min().unwrap_or(0);
    if n == 0 {
        return Ok(Vec::new());
    }

    let bytes_per_frame = nch * 4;
    let max_samples_per_chunk = MAX_AUDIO_BYTES / bytes_per_frame;
    if max_samples_per_chunk == 0 {
        return Err("too many channels for one ReaStream block".into());
    }

    let mut id_pad = [0u8; 32];
    id_pad[..identifier.len()].copy_from_slice(identifier.as_bytes());

    let mut packets = Vec::new();
    let mut offset = 0usize;
    while offset < n {
        let chunk = (n - offset).min(max_samples_per_chunk);
        let sblocklen = chunk * bytes_per_frame;
        let packet_size = 4 + 4 + 32 + 1 + 4 + 2 + sblocklen;
        let mut buf = Vec::with_capacity(packet_size);
        buf.extend_from_slice(MAGIC);
        buf.extend_from_slice(&(packet_size as u32).to_le_bytes());
        buf.extend_from_slice(&id_pad);
        buf.push(nch as u8);
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&(sblocklen as u16).to_le_bytes());
        for ch in channels {
            for sample in &ch[offset..offset + chunk] {
                buf.extend_from_slice(&sample.to_le_bytes());
            }
        }
        debug_assert_eq!(buf.len(), packet_size);
        packets.push(buf);
        offset += chunk;
    }
    Ok(packets)
}

pub fn send_packets(socket: &UdpSocket, target: SocketAddr, packets: &[Vec<u8>]) {
    for packet in packets {
        let _ = socket.send_to(packet, target);
    }
}

const QUEUE_CAP_SAMPLES: usize = 64_000;
pub const JITTER_BUFFER_MS_DEFAULT: u32 = 30;

struct ReaStreamQueue {
    channels: Vec<VecDeque<f32>>,
    finite: bool,
    epoch: u64,
}

impl ReaStreamQueue {
    fn push_block(&mut self, channel_samples: &[&[f32]]) {
        let nch = channel_samples.len();
        if self.channels.len() != nch {
            self.channels = (0..nch).map(|_| VecDeque::new()).collect();
        }
        let n = channel_samples.iter().map(|c| c.len()).min().unwrap_or(0);
        for (i, ch) in channel_samples.iter().enumerate() {
            let q = &mut self.channels[i];
            q.extend(ch.iter().take(n).copied());
            while q.len() > QUEUE_CAP_SAMPLES {
                q.pop_front();
            }
        }
        self.finite = false;
    }

    fn replace_block(&mut self, channel_samples: &[&[f32]]) {
        let n = channel_samples
            .iter()
            .map(|c| c.len())
            .min()
            .unwrap_or(0)
            .min(QUEUE_CAP_SAMPLES);
        self.channels = channel_samples
            .iter()
            .map(|ch| ch.iter().take(n).copied().collect())
            .collect();
        self.finite = true;
        self.epoch = self.epoch.wrapping_add(1);
    }

    fn clear(&mut self) {
        for ch in &mut self.channels {
            ch.clear();
        }
        self.finite = false;
        self.epoch = self.epoch.wrapping_add(1);
    }

    fn take_send_block(&mut self, limit: usize) -> Option<Vec<Vec<f32>>> {
        if self.channels.is_empty() || limit == 0 {
            return None;
        }
        let nch = self.channels.len();
        let bytes_per_frame = nch * 4;
        let max_take = MAX_AUDIO_BYTES / bytes_per_frame;
        if max_take == 0 {
            return None;
        }
        let available = self.channels.iter().map(|c| c.len()).min().unwrap_or(0);
        let take = available.min(max_take).min(limit);
        if take == 0 {
            return None;
        }
        let mut bufs = Vec::with_capacity(nch);
        for ch in &mut self.channels {
            bufs.push(ch.drain(..take).collect());
        }
        Some(bufs)
    }

    fn pending_samples(&self) -> usize {
        self.channels.iter().map(|c| c.len()).min().unwrap_or(0)
    }
}

/// Background UDP sender: cooks enqueue samples; this thread drains at its own pace.
pub struct ReaStreamSender {
    queue: Arc<Mutex<ReaStreamQueue>>,
    sample_rate: Arc<AtomicU32>,
    jitter_ms: Arc<AtomicU32>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
}

impl Drop for ReaStreamSender {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

impl ReaStreamSender {
    pub fn open(
        socket: UdpSocket,
        target: SocketAddr,
        identifier: String,
        sample_rate: u32,
        channels: usize,
        jitter_ms: u32,
    ) -> Result<Self, String> {
        let jitter_ms = if jitter_ms == 0 {
            JITTER_BUFFER_MS_DEFAULT
        } else {
            jitter_ms
        };
        let queue = Arc::new(Mutex::new(ReaStreamQueue {
            channels: (0..channels.max(1)).map(|_| VecDeque::new()).collect(),
            finite: false,
            epoch: 0,
        }));
        let rate = Arc::new(AtomicU32::new(sample_rate));
        let jitter = Arc::new(AtomicU32::new(jitter_ms.max(1)));
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let queue_in = queue.clone();
        let rate_in = rate.clone();
        let jitter_in = jitter.clone();
        let thread = std::thread::Builder::new()
            .name(format!("otd-reastream-{}", identifier))
            .spawn(move || {
                crate::thread_qos::promote_current_thread_interactive();
                let mut start = std::time::Instant::now();
                let mut samples_sent: u64 = 0;
                let mut local_epoch = 0u64;
                let mut clock_on = false;
                while !stop_flag.load(Ordering::Relaxed) {
                    let rate_val = rate_in.load(Ordering::Relaxed).max(1);
                    let jitter_ms = jitter_in.load(Ordering::Relaxed).max(1);
                    let jitter_samples =
                        (u64::from(rate_val) * u64::from(jitter_ms) / 1000).max(64);
                    let ready = {
                        let Ok(q) = queue_in.lock() else {
                            return;
                        };
                        if q.epoch != local_epoch {
                            local_epoch = q.epoch;
                            clock_on = false;
                            samples_sent = 0;
                        }
                        let pending = q.pending_samples() as u64;
                        pending > 0 && (q.finite || pending >= jitter_samples)
                    };
                    if !ready {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        continue;
                    }
                    if !clock_on {
                        start = std::time::Instant::now();
                        samples_sent = 0;
                        clock_on = true;
                    }
                    let elapsed = start.elapsed().as_secs_f64();
                    let budget = (elapsed * f64::from(rate_val)).floor() as u64 + jitter_samples;
                    let ahead_limit = budget.saturating_sub(jitter_samples / 2);
                    if samples_sent >= ahead_limit {
                        let wake_at = samples_sent.saturating_sub(jitter_samples / 2) as f64
                            / f64::from(rate_val);
                        let wait = (wake_at - elapsed).clamp(1.0 / f64::from(rate_val), 0.05);
                        std::thread::sleep(std::time::Duration::from_secs_f64(wait));
                        continue;
                    }
                    let due = (ahead_limit - samples_sent) as usize;
                    let block = {
                        let Ok(mut q) = queue_in.lock() else {
                            return;
                        };
                        if q.epoch != local_epoch {
                            continue;
                        }
                        let pending = q.pending_samples();
                        if pending == 0 || (!q.finite && (pending as u64) < jitter_samples) {
                            None
                        } else {
                            q.take_send_block(due)
                        }
                    };
                    let Some(bufs) = block else {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                        continue;
                    };
                    let chunk = bufs.first().map(|b| b.len()).unwrap_or(0) as u64;
                    let refs: Vec<&[f32]> = bufs.iter().map(|s| s.as_slice()).collect();
                    if let Ok(packets) = encode_audio_packets(&identifier, rate_val, &refs) {
                        send_packets(&socket, target, &packets);
                        samples_sent += chunk;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(ReaStreamSender {
            queue,
            sample_rate: rate,
            jitter_ms: jitter,
            stop,
            _thread: Some(thread),
        })
    }

    pub fn set_sample_rate(&self, rate: u32) {
        self.sample_rate.store(rate, Ordering::Relaxed);
    }

    pub fn set_jitter_ms(&self, ms: u32) {
        self.jitter_ms.store(ms.max(1), Ordering::Relaxed);
    }

    pub fn clear(&self) {
        let Ok(mut q) = self.queue.lock() else {
            return;
        };
        q.clear();
    }

    pub fn push(&self, channel_samples: &[&[f32]]) {
        let Ok(mut q) = self.queue.lock() else {
            return;
        };
        q.push_block(channel_samples);
    }

    pub fn replace(&self, channel_samples: &[&[f32]]) {
        let Ok(mut q) = self.queue.lock() else {
            return;
        };
        q.replace_block(channel_samples);
    }
}

#[cfg(test)]
#[path = "tests/reastream__tests.rs"]
mod tests;
