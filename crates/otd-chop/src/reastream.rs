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
const HEADER_BYTES: usize = 4 + 4 + 32 + 1 + 4 + 2;
pub const MAX_CHANNELS: usize = 64;

/// `host:58710`, with `localhost` and an empty field meaning this machine.
pub fn target(host: &str) -> Result<SocketAddr, String> {
    let host = host.trim();
    let host = if host.eq_ignore_ascii_case("localhost") || host.is_empty() {
        "127.0.0.1"
    } else {
        host
    };
    format!("{host}:{DEFAULT_PORT}")
        .parse()
        .map_err(|_| format!("bad ReaStream address `{host}`"))
}

fn identifier_pad(identifier: &str) -> Result<[u8; 32], String> {
    if identifier.len() >= 32 {
        return Err("identifier must be shorter than 32 characters".into());
    }
    let mut id_pad = [0u8; 32];
    id_pad[..identifier.len()].copy_from_slice(identifier.as_bytes());
    Ok(id_pad)
}

fn frames_per_packet(nch: usize) -> Result<usize, String> {
    if nch > MAX_CHANNELS {
        return Err(format!(
            "ReaStream supports at most {MAX_CHANNELS} channels (got {nch})"
        ));
    }
    match MAX_AUDIO_BYTES / (nch.max(1) * 4) {
        0 => Err("too many channels for one ReaStream block".into()),
        n => Ok(n),
    }
}

fn write_packet(
    buf: &mut Vec<u8>,
    id_pad: &[u8; 32],
    sample_rate: u32,
    channels: &[&[f32]],
    offset: usize,
    chunk: usize,
) {
    let sblocklen = chunk * channels.len() * 4;
    buf.clear();
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&((HEADER_BYTES + sblocklen) as u32).to_le_bytes());
    buf.extend_from_slice(id_pad);
    buf.push(channels.len() as u8);
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sblocklen as u16).to_le_bytes());
    for ch in channels {
        for sample in &ch[offset..offset + chunk] {
            buf.extend_from_slice(&sample.to_le_bytes());
        }
    }
}

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
    let per_packet = frames_per_packet(channels.len())?;
    let id_pad = identifier_pad(identifier)?;
    let n = channels.iter().map(|c| c.len()).min().unwrap_or(0);
    let mut packets = Vec::new();
    let mut offset = 0usize;
    while offset < n {
        let chunk = (n - offset).min(per_packet);
        let mut buf = Vec::with_capacity(HEADER_BYTES + chunk * channels.len() * 4);
        write_packet(&mut buf, &id_pad, sample_rate, channels, offset, chunk);
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

/// A non-blocking ReaStream socket for a caller that already runs at the
/// sample clock: every block goes out as soon as it is rendered, so the
/// stream is paced by whoever renders it and never by a second timer.
pub struct ReaStreamLink {
    socket: UdpSocket,
    target: SocketAddr,
    id_pad: [u8; 32],
    sample_rate: u32,
}

impl ReaStreamLink {
    pub fn open(host: &str, identifier: &str, sample_rate: u32) -> Result<Self, String> {
        let target = target(host)?;
        let id_pad = identifier_pad(identifier)?;
        let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0)))
            .map_err(|e| format!("ReaStream bind: {e}"))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| format!("ReaStream socket: {e}"))?;
        Ok(Self {
            socket,
            target,
            id_pad,
            sample_rate: sample_rate.max(1),
        })
    }

    /// Sends one block; `packet` is reused between calls. False when the
    /// network refused a packet, which is a dropped block, not a stall.
    pub fn send(&self, channels: &[&[f32]], packet: &mut Vec<u8>) -> bool {
        let Ok(per_packet) = frames_per_packet(channels.len()) else {
            return false;
        };
        let n = channels.iter().map(|c| c.len()).min().unwrap_or(0);
        let mut offset = 0usize;
        let mut sent = true;
        while offset < n {
            let chunk = (n - offset).min(per_packet);
            write_packet(
                packet,
                &self.id_pad,
                self.sample_rate,
                channels,
                offset,
                chunk,
            );
            sent &= self.socket.send_to(packet, self.target).is_ok();
            offset += chunk;
        }
        sent
    }
}

const QUEUE_CAP_SAMPLES: usize = 64_000;
pub const JITTER_BUFFER_MS_DEFAULT: u32 = 30;

enum Pace {
    Buffer,
    Wait(f64),
    Send(usize),
    Resync,
}

fn lead_samples(rate: u32, jitter_samples: u64) -> u64 {
    let twenty_ms = u64::from(rate.max(1)).saturating_mul(20) / 1000;
    twenty_ms.clamp(64, jitter_samples.max(64) / 2)
}

fn cushion_samples(jitter_samples: u64, last_block: usize, rate: u32) -> u64 {
    let from_block = (last_block as u64).saturating_mul(2);
    let one_sec = u64::from(rate.max(1));
    jitter_samples.max(from_block).clamp(64, one_sec)
}

fn pace(
    playing: bool,
    samples_sent: u64,
    elapsed_secs: f64,
    pending: usize,
    rate: u32,
    cushion: u64,
    finite: bool,
) -> Pace {
    let lead = lead_samples(rate, cushion);
    let rate_f = f64::from(rate.max(1));
    if !playing {
        let primed = pending > 0 && (finite || pending as u64 >= cushion);
        return if primed {
            Pace::Send(lead as usize)
        } else {
            Pace::Buffer
        };
    }
    let elapsed_samples = (elapsed_secs.max(0.0) * rate_f).floor() as u64;
    if !finite && elapsed_samples.saturating_sub(samples_sent) > cushion {
        return Pace::Resync;
    }
    if pending == 0 {
        return Pace::Wait(0.001);
    }
    let target = elapsed_samples.saturating_add(lead);
    if samples_sent >= target {
        let wake_at = samples_sent.saturating_sub(lead) as f64 / rate_f;
        let wait = (wake_at - elapsed_secs).clamp(1.0 / rate_f, 0.005);
        return Pace::Wait(wait);
    }
    Pace::Send((target - samples_sent) as usize)
}

struct ReaStreamQueue {
    channels: Vec<VecDeque<f32>>,
    finite: bool,
    epoch: u64,
    last_block: usize,
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
        self.last_block = n;
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
        self.last_block = n;
        self.finite = true;
        self.epoch = self.epoch.wrapping_add(1);
    }

    fn clear(&mut self) {
        for ch in &mut self.channels {
            ch.clear();
        }
        self.last_block = 0;
        self.finite = false;
        self.epoch = self.epoch.wrapping_add(1);
    }

    fn trim_to(&mut self, keep: usize) {
        for ch in &mut self.channels {
            while ch.len() > keep {
                ch.pop_front();
            }
        }
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
            last_block: 0,
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
                    let (pending, finite, last_block) = {
                        let Ok(q) = queue_in.lock() else {
                            return;
                        };
                        if q.epoch != local_epoch {
                            local_epoch = q.epoch;
                            clock_on = false;
                            samples_sent = 0;
                        }
                        (q.pending_samples(), q.finite, q.last_block)
                    };
                    let cushion = cushion_samples(jitter_samples, last_block, rate_val);
                    let elapsed = if clock_on {
                        start.elapsed().as_secs_f64()
                    } else {
                        0.0
                    };
                    match pace(
                        clock_on,
                        samples_sent,
                        elapsed,
                        pending,
                        rate_val,
                        cushion,
                        finite,
                    ) {
                        Pace::Buffer => {
                            std::thread::sleep(std::time::Duration::from_millis(1));
                            continue;
                        }
                        Pace::Resync => {
                            start = std::time::Instant::now();
                            samples_sent = 0;
                            clock_on = true;
                            if let Ok(mut q) = queue_in.lock() {
                                q.trim_to(cushion as usize);
                            }
                            continue;
                        }
                        Pace::Wait(wait) => {
                            std::thread::sleep(std::time::Duration::from_secs_f64(wait));
                            continue;
                        }
                        Pace::Send(due) => {
                            if !clock_on {
                                start = std::time::Instant::now();
                                samples_sent = 0;
                                clock_on = true;
                            }
                            let block = {
                                let Ok(mut q) = queue_in.lock() else {
                                    return;
                                };
                                if q.epoch != local_epoch {
                                    continue;
                                }
                                if q.pending_samples() == 0 {
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
                            if let Ok(packets) = encode_audio_packets(&identifier, rate_val, &refs)
                            {
                                send_packets(&socket, target, &packets);
                                samples_sent += chunk;
                            }
                        }
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
