use std::collections::HashMap;
use std::sync::Arc;

use otd_audio::{AudioWorker, TapReader, tap_channel};
use otd_chop::data::ChopData;
use otd_chop::engine::{ChannelStore, Network};
use otd_chop::io::Io;
use otd_chop::rt::chop_from_interleaved;
use otd_chop::rt::plan_build::{PlanBuild, SinkKind, build_audio_plan, plan_signature};
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
    known_channels: HashMap<NodeId, usize>,
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
            known_channels: HashMap::new(),
        }
    }

    pub fn is_armed(&self) -> bool {
        self.worker.is_some() && self.live.is_some()
    }

    pub fn device_sample_rate(&self) -> Option<f64> {
        self.worker.as_ref().map(AudioWorker::sample_rate)
    }

    pub fn underruns(&self) -> u64 {
        self.worker
            .as_ref()
            .map(AudioWorker::underruns)
            .unwrap_or(0)
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
        if partition.sinks.is_empty() {
            self.stop();
            return;
        }
        self.learn_channels(partition, channels);
        let known = &self.known_channels;
        let count = |id: NodeId| known.get(&id).copied();
        let signature = plan_signature(
            graph,
            &partition.sinks,
            &partition.audio_nodes,
            &partition.bridge_nodes,
            &count,
        );
        if self.signature != Some(signature) {
            self.recompile(graph, partition, chop_io, sample_rate);
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
        share.publish_channels(Arc::new(channels.clone()));
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
        sync_sinks(graph, time, channels, chop_io, &mut live.build, stream_rate);
        self.drain_tap(channels);
        if let Some(worker) = self.worker.as_mut() {
            worker.collect_retired();
        }
    }

    /// Channel layouts the plan is sized to: what the frame cook produced, and
    /// over it what the worker really cooked. Kept across recompiles so a
    /// fresh plan that has not rendered yet does not trigger another one.
    fn learn_channels(&mut self, partition: &AudioPartition, channels: &ChannelStore) {
        for id in partition.audio_nodes.iter().chain(&partition.bridge_nodes) {
            let worker = self
                .live
                .as_ref()
                .and_then(|l| l.build.produced_channels(*id));
            let frame = channels
                .get(*id)
                .map(ChopData::num_channels)
                .filter(|c| *c > 0);
            if let Some(count) = worker.or(frame) {
                self.known_channels.insert(*id, count);
            }
        }
    }

    fn recompile(
        &mut self,
        graph: &Graph,
        partition: &AudioPartition,
        chop_io: &Io,
        sample_rate: f64,
    ) {
        let known = &self.known_channels;
        let count = |id: NodeId| known.get(&id).copied();
        let Some((mut plan, build)) = build_audio_plan(
            graph,
            &partition.sinks,
            &partition.audio_nodes,
            &partition.bridge_nodes,
            &count,
        ) else {
            self.stop();
            return;
        };
        let rate = otd_core::canonical_project_sample_rate(sample_rate);
        let key = OutputKey {
            device: build.device().map(str::to_string),
            channels: build.channels,
            rate: rate.round() as u32,
            latency_ms: chop_io.audio_latency_ms(),
        };
        if self.output.as_ref() != Some(&key) || self.worker.is_none() {
            self.worker = None;
            self.tap = None;
            let tap_len = (rate * TAP_SECONDS) as usize * build.tap_channels.max(1);
            let (tap_tx, mut tap_rx) = tap_channel(tap_len);
            let device = key.device.as_deref().unwrap_or("");
            let started =
                AudioWorker::start_device(device, key.channels, rate, key.latency_ms, Some(tap_tx))
                    .and_then(|worker| {
                        let off_rate = (worker.sample_rate() - rate).abs() > 0.5;
                        if key.device.is_none() && off_rate {
                            return Err(format!(
                                "default output runs at {} Hz, project rate is {rate} Hz",
                                worker.sample_rate()
                            ));
                        }
                        Ok(worker)
                    })
                    .or_else(|e| {
                        if key.device.is_some() {
                            return Err(e);
                        }
                        log::warn!(
                            "audio runtime: not clocking on the output device ({e}), using a timer"
                        );
                        let (tap_tx, rx) = tap_channel(tap_len);
                        tap_rx = rx;
                        AudioWorker::start_clock(rate, key.channels, Some(tap_tx))
                    });
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
        let (Some(tap), Some(live), Some(worker)) = (self.tap.as_mut(), &self.live, &self.worker)
        else {
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

fn sync_sinks(
    graph: &Graph,
    time: &CookContext,
    channels: &ChannelStore,
    chop_io: &mut Io,
    build: &mut PlanBuild,
    sample_rate: f64,
) {
    let net = Network::new(graph, channels);
    let rate = otd_core::canonical_project_sample_rate(sample_rate)
        .round()
        .max(1.0) as u32;
    let mut streamed = Vec::new();
    for sink in &mut build.sinks {
        let Some(node) = graph.get(sink.id) else {
            continue;
        };
        let path = graph.path(sink.id);
        let (t, abs, frame) = graph.local_clock_at(sink.id, time);
        let eval = EvalContext {
            frame,
            time: t,
            abs_time: abs,
            fps: time.fps,
            channels: Some(&net),
            path: Some(&path),
            locals: &[],
        };
        let param = |key: &str, fallback: &str| {
            node.params
                .get(key)
                .map(|p| p.eval(&eval).as_str().to_string())
                .unwrap_or_else(|| fallback.to_string())
        };
        let active = node
            .params
            .get("active")
            .is_none_or(|p| p.eval(&eval).as_bool());
        match &mut sink.kind {
            SinkKind::Device { volume, .. } => {
                let v = node
                    .params
                    .get("volume")
                    .map(|p| p.eval(&eval).as_f32())
                    .unwrap_or(1.0);
                volume.set(if active { v } else { 0.0 });
            }
            SinkKind::Reaper(feed) => {
                let host = param("netaddress", "127.0.0.1");
                let identifier = param("identifier", "default");
                match feed.sync(&host, &identifier, rate, active) {
                    Some(error) => chop_io.note(&path, error),
                    None => chop_io.clear_note(&path),
                }
                streamed.push(path);
            }
        }
    }
    chop_io.release_frame_reaper_outs(&streamed);
}
