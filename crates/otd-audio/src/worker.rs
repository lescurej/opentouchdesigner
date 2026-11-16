use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SupportedStreamConfig;
use rtrb::RingBuffer;

use crate::executor::AudioExecutor;
use crate::plan::AudioPlan;
use crate::swap::{plan_channel, PlanSender};
use crate::tap::TapWriter;
use crate::thread_qos::promote_current_thread_interactive;

pub const BLOCK_FRAMES: usize = 256;
const PLAN_QUEUE: usize = 4;

/// Owns the thread that renders plans. With a device, the worker renders
/// ahead into a lock-free ring and the device callback only copies samples
/// out of it; without one, the worker paces itself on a wall clock.
pub struct AudioWorker {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    _stream: Option<cpal::Stream>,
    plans: PlanSender,
    sample_rate: f64,
    channels: usize,
    underruns: Arc<AtomicU64>,
}

impl AudioWorker {
    pub fn start_device(
        device_name: &str,
        want_channels: usize,
        want_rate: f64,
        latency_ms: u32,
        tap: Option<TapWriter>,
    ) -> Result<Self, String> {
        let device = find_output_device(device_name)?;
        let config = pick_f32_output(&device, want_channels, want_rate.round().max(1.0) as u32)?;
        let channels = config.channels().max(1) as usize;
        let sample_rate = f64::from(config.sample_rate());
        let target_frames =
            ((sample_rate * f64::from(latency_ms.max(1)) / 1000.0) as usize).max(BLOCK_FRAMES * 2);
        let (mut producer, mut consumer) = RingBuffer::<f32>::new(target_frames * channels);
        let underruns = Arc::new(AtomicU64::new(0));
        let underruns_cb = underruns.clone();
        let started = Arc::new(AtomicBool::new(false));
        let started_cb = started.clone();
        let stream = device
            .build_output_stream(
                config.config(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let (filled, rest) = consumer.pop_partial_slice(data);
                    let got = filled.len();
                    if !rest.is_empty() {
                        rest.fill(0.0);
                        if started_cb.load(Ordering::Relaxed) && got > 0 {
                            underruns_cb.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                },
                |err| log::warn!("audio output error: {err}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        let (plans, rx) = plan_channel(PLAN_QUEUE);
        let mut executor = AudioExecutor::new(Box::new(AudioPlan::empty(BLOCK_FRAMES)), Some(rx), tap);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_w = stop.clone();
        let block_samples = BLOCK_FRAMES * channels;
        let nap = Duration::from_secs_f64(BLOCK_FRAMES as f64 / sample_rate / 4.0);
        let join = thread::Builder::new()
            .name("otd-audio-worker".into())
            .spawn(move || {
                promote_current_thread_interactive();
                let mut scratch = vec![0.0f32; block_samples];
                while !stop_w.load(Ordering::Relaxed) {
                    while producer.slots() >= block_samples {
                        executor.render(&mut scratch, channels);
                        let _ = producer.push_partial_slice(&scratch);
                        started.store(true, Ordering::Relaxed);
                    }
                    thread::sleep(nap);
                }
            })
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Self {
            stop,
            join: Some(join),
            _stream: Some(stream),
            plans,
            sample_rate,
            channels,
            underruns,
        })
    }

    pub fn start_clock(sample_rate: f64, channels: usize, tap: Option<TapWriter>) -> Result<Self, String> {
        let sample_rate = sample_rate.max(1.0);
        let channels = channels.max(1);
        let (plans, rx) = plan_channel(PLAN_QUEUE);
        let mut executor = AudioExecutor::new(Box::new(AudioPlan::empty(BLOCK_FRAMES)), Some(rx), tap);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_w = stop.clone();
        let underruns = Arc::new(AtomicU64::new(0));
        let late = underruns.clone();
        let period = Duration::from_secs_f64(BLOCK_FRAMES as f64 / sample_rate);
        let join = thread::Builder::new()
            .name("otd-audio-clock".into())
            .spawn(move || {
                promote_current_thread_interactive();
                let mut scratch = vec![0.0f32; BLOCK_FRAMES * channels];
                let mut deadline = Instant::now();
                while !stop_w.load(Ordering::Relaxed) {
                    executor.render(&mut scratch, channels);
                    deadline += period;
                    let now = Instant::now();
                    if deadline > now {
                        thread::sleep(deadline - now);
                    } else if now - deadline > period * 4 {
                        late.fetch_add(1, Ordering::Relaxed);
                        deadline = now;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            stop,
            join: Some(join),
            _stream: None,
            plans,
            sample_rate,
            channels,
            underruns,
        })
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// `plan` must already be prepared at [`Self::sample_rate`].
    pub fn publish(&mut self, plan: Box<AudioPlan>) -> Result<(), Box<AudioPlan>> {
        self.plans.publish(plan)
    }

    pub fn collect_retired(&mut self) {
        self.plans.collect_retired();
    }
}

impl Drop for AudioWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

fn find_output_device(name: &str) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    if name.trim().is_empty() {
        return host
            .default_output_device()
            .ok_or_else(|| "no default audio output device".to_string());
    }
    host.output_devices()
        .map_err(|e| e.to_string())?
        .find(|d| d.description().ok().is_some_and(|desc| desc.name() == name))
        .ok_or_else(|| format!("no audio output device called `{name}`"))
}

fn pick_f32_output(
    device: &cpal::Device,
    want_channels: usize,
    want_rate: u32,
) -> Result<SupportedStreamConfig, String> {
    let ranges: Vec<_> = device
        .supported_output_configs()
        .map_err(|e| e.to_string())?
        .filter(|r| r.sample_format() == cpal::SampleFormat::F32)
        .collect();
    if ranges.is_empty() {
        return Err("audio output device has no f32 configuration".into());
    }
    let max_ch = ranges.iter().map(|r| r.channels() as usize).max().unwrap_or(1);
    let target = want_channels.clamp(1, max_ch) as u16;
    let pool: Vec<_> = ranges.iter().filter(|r| r.channels() == target).collect();
    let pool = if pool.is_empty() { ranges.iter().collect() } else { pool };
    let range = pool
        .iter()
        .find(|r| r.min_sample_rate() <= want_rate && want_rate <= r.max_sample_rate())
        .or_else(|| pool.first())
        .ok_or("no output configuration")?;
    let rate = want_rate.clamp(range.min_sample_rate(), range.max_sample_rate());
    Ok(range.with_sample_rate(rate))
}
