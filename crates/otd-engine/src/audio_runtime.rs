use std::sync::Arc;

use otd_audio::{tap_channel, AudioWorker, TapReader};
use otd_chop::data::ChopData;
use otd_chop::engine::ChannelStore;
use otd_chop::io::Io;
use otd_chop::rt::plan_build::{build_audio_plan, plan_signature, PlanBuild};
use otd_chop::rt::chop_from_interleaved;
use otd_core::{CookContext, EvalContext, Graph, NodeId};

use crate::audio_partition::AudioPartition;

const TAP_SECONDS: f64 = 1.0;

#[derive(Clone, PartialEq, Debug)]
struct OutputKey {
    device: Option<String>,
    channels: usize,
    rate: u32,
    latency_ms: u32,
}

struct Live {
    build: PlanBuild,
    bridge_seen: Vec<Option<Arc<ChopData>>>,
}

/// Frame-thread owner of the audio worker: compiles the audio partition into
/// plans, (re)opens the output when the device or layout changes, and feeds
/// the worker parameters, bridge samples and the frame clock.
pub struct AudioRuntime {
    worker: Option<AudioWorker>,
    output: Option<OutputKey>,
    live: Option<Live>,
    signature: Option<u64>,
    graph_content: Option<u64>,
    tap: Option<TapReader>,
    tap_data: Option<(NodeId, Arc<ChopData>)>,
    tap_scratch: Vec<f32>,
}

impl Default for AudioRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioRuntime {
    pub fn new() -> Self {
        Self {
            worker: None,
            output: None,
            live: None,
            signature: None,
            graph_content: None,
            tap: None,
            tap_data: None,
            tap_scratch: Vec::new(),
        }
    }

    pub fn is_armed(&self) -> bool {
        self.worker.is_some() && self.live.is_some()
    }

    pub fn device_sample_rate(&self) -> Option<f64> {
        self.worker.as_ref().map(AudioWorker::sample_rate)
    }

    pub fn underruns(&self) -> u64 {
        self.worker.as_ref().map(AudioWorker::underruns).unwrap_or(0)
    }

    pub fn tap_chop(&self, id: NodeId) -> Option<Arc<ChopData>> {
        self.tap_data
            .as_ref()
            .filter(|(tap, _)| *tap == id)
            .map(|(_, data)| data.clone())
    }

    pub fn tick(
        &mut self,
        graph: &Graph,
        partition: &AudioPartition,
        time: &CookContext,
        channels: &mut ChannelStore,
        chop_io: &mut Io,
        sample_rate: f64,
    ) {
        if partition.audio_nodes.is_empty() {
            self.stop();
            return;
        }
        let signature = plan_signature(graph, &partition.audio_nodes, &partition.bridge_nodes);
        if self.signature != Some(signature) {
            self.recompile(graph, partition, channels, chop_io, sample_rate);
            self.signature = Some(signature);
        }
        let Some(live) = self.live.as_mut() else {
            return;
        };
        let share = &live.build.share;
        if self.graph_content != Some(partition.generation) {
            share.publish_graph(Arc::new(graph.clone()));
            self.graph_content = Some(partition.generation);
        }
        share.publish_time(time);
        share.publish_io(chop_io);
        for ((id, feed), seen) in live.build.bridges.iter_mut().zip(&mut live.bridge_seen) {
            let Some(data) = channels.arc(*id) else {
                continue;
            };
            if seen.as_ref().is_some_and(|s| Arc::ptr_eq(s, &data)) {
                continue;
            }
            feed.publish(&data);
            *seen = Some(data);
        }
        let stream_rate = self
            .worker
            .as_ref()
            .map(otd_audio::AudioWorker::sample_rate)
            .unwrap_or(sample_rate);
        sync_sink(graph, time, chop_io, &live.build, stream_rate);
        self.drain_tap(channels);
        if let Some(worker) = self.worker.as_mut() {
            worker.collect_retired();
        }
    }

    fn recompile(
        &mut self,
        graph: &Graph,
        partition: &AudioPartition,
        channels: &ChannelStore,
        chop_io: &Io,
        sample_rate: f64,
    ) {
        let count = |id: NodeId| channels.get(id).map(ChopData::num_channels);
        let Some((mut plan, build)) =
            build_audio_plan(graph, &partition.audio_nodes, &partition.bridge_nodes, &count)
        else {
            self.stop();
            return;
        };
        let rate = otd_core::canonical_project_sample_rate(sample_rate);
        let key = OutputKey {
            device: build.sink_is_device.then(|| build.device.clone()),
            channels: build.channels,
            rate: rate.round() as u32,
            latency_ms: chop_io.audio_latency_ms(),
        };
        if self.output.as_ref() != Some(&key) || self.worker.is_none() {
            self.worker = None;
            self.tap = None;
            let (tap_tx, tap_rx) =
                tap_channel((rate * TAP_SECONDS) as usize * build.tap_channels.max(1));
            let started = match &key.device {
                Some(device) => {
                    AudioWorker::start_device(device, key.channels, rate, key.latency_ms, Some(tap_tx))
                }
                None => AudioWorker::start_clock(rate, key.channels, Some(tap_tx)),
            };
            match started {
                Ok(worker) => {
                    if (worker.sample_rate() - rate).abs() > 0.5 {
                        log::warn!(
                            "audio output runs at {} Hz, project rate is {rate} Hz",
                            worker.sample_rate()
                        );
                    }
                    self.worker = Some(worker);
                    self.tap = Some(tap_rx);
                    self.output = Some(key);
                }
                Err(e) => {
                    log::warn!("audio runtime: {e}");
                    self.stop();
                    return;
                }
            }
        }
        let Some(worker) = self.worker.as_mut() else {
            return;
        };
        plan.prepare(worker.sample_rate());
        if let Err(plan) = worker.publish(Box::new(plan)) {
            log::warn!("audio runtime: plan queue full, dropping {plan:?}");
            return;
        }
        self.graph_content = None;
        self.tap_data = None;
        self.live = Some(Live {
            bridge_seen: vec![None; build.bridges.len()],
            build,
        });
    }

    fn drain_tap(&mut self, channels: &mut ChannelStore) {
        let (Some(tap), Some(live), Some(worker)) = (self.tap.as_mut(), &self.live, &self.worker) else {
            return;
        };
        self.tap_scratch.clear();
        if tap.drain_into(&mut self.tap_scratch) > 0 {
            let data = chop_from_interleaved(
                &self.tap_scratch,
                live.build.tap_channels,
                worker.sample_rate(),
            );
            self.tap_data = Some((live.build.tap_node, Arc::new(data)));
        }
        if let Some((id, data)) = &self.tap_data {
            channels.insert(*id, data.clone());
        }
    }

    pub fn stop(&mut self) {
        self.live = None;
        self.worker = None;
        self.output = None;
        self.tap = None;
        self.tap_data = None;
        self.signature = None;
        self.graph_content = None;
    }
}

fn eval_for<'a>(graph: &Graph, id: NodeId, time: &CookContext, path: &'a str) -> EvalContext<'a> {
    let (t, abs, frame) = graph.local_clock_at(id, time);
    EvalContext {
        frame,
        time: t,
        abs_time: abs,
        fps: time.fps,
        channels: None,
        path: Some(path),
        locals: &[],
    }
}

fn sync_sink(
    graph: &Graph,
    time: &CookContext,
    chop_io: &mut Io,
    build: &PlanBuild,
    sample_rate: f64,
) {
    let Some(node) = graph.get(build.sink_id) else {
        return;
    };
    let path = graph.path(build.sink_id);
    let eval = eval_for(graph, build.sink_id, time, &path);
    let param = |key: &str| node.params.get(key).map(|p| p.eval(&eval));
    let active = param("active").is_none_or(|v| v.as_bool());
    if let Some(volume) = &build.sink_volume {
        let v = param("volume").map(|v| v.as_f32()).unwrap_or(1.0);
        volume.set(if active { v } else { 0.0 });
    }
    let Some(slot) = &build.reaper_slot else {
        return;
    };
    let Ok(mut slot) = slot.lock() else {
        return;
    };
    slot.active = active;
    let net = param("netaddress").map(|v| v.as_str().to_string()).unwrap_or_else(|| "127.0.0.1".into());
    let identifier = param("identifier").map(|v| v.as_str().to_string()).unwrap_or_else(|| "default".into());
    let rate = otd_core::canonical_project_sample_rate(sample_rate).round().max(1.0) as u32;
    let result = otd_chop::io::ensure_rt_reaper_sender(
        &mut slot,
        &net,
        &identifier,
        rate,
        build.channels,
        chop_io.reastream_jitter_ms(),
    );
    let error = result.err();
    if error.is_some() && error != slot.error {
        log::warn!("ReaStream: {}", error.as_deref().unwrap_or_default());
    }
    slot.error = error;
    if let Some(msg) = slot.error.as_deref() {
        chop_io.note(&path, msg);
    } else if slot.active && slot.sender.is_some() {
        chop_io.clear_note(&path);
    }
}
