use std::collections::{HashMap, HashSet};

use otd_chop::ChopData;
use otd_core::NodeId;
use otd_engine::{NodeMessage, ProjectRuntime};

use super::OtdApp;

#[derive(Default)]
pub(crate) struct EngineUiSnapshot {
    pub frame: i64,
    pub last_cook_us: HashMap<NodeId, u64>,
    pub time_dependent: HashSet<NodeId>,
    pub output_version: HashMap<NodeId, u64>,
    pub top_meta: HashMap<NodeId, (u32, u32)>,
    pub chop: HashMap<NodeId, ChopData>,
    pub node_messages: HashMap<NodeId, NodeMessage>,
    pub avg_cook_ms: HashMap<NodeId, f64>,
    pub cook_rate: HashMap<NodeId, f64>,
    pub has_cooked: HashSet<NodeId>,
    pub cooked_this_frame: HashSet<NodeId>,
}

impl OtdApp {
    pub(crate) fn rebuild_engine_ui_snapshot(&mut self) {
        let Ok(rt) = self.engine.runtime.lock() else {
            return;
        };
        refresh_snapshot(&mut self.ui_snapshot, &rt, &self.graph, self.time.frame);
    }

    pub(crate) fn snapshot_last_cook_us(&self, id: NodeId) -> u64 {
        self.ui_snapshot.last_cook_us.get(&id).copied().unwrap_or(0)
    }

    pub(crate) fn snapshot_time_dependent(&self, id: NodeId) -> bool {
        self.ui_snapshot.time_dependent.contains(&id)
    }

    pub(crate) fn snapshot_output_version(&self, id: NodeId) -> u64 {
        self.ui_snapshot
            .output_version
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    pub(crate) fn snapshot_top_meta(&self, id: NodeId) -> Option<(u32, u32)> {
        self.ui_snapshot.top_meta.get(&id).copied()
    }

    pub(crate) fn snapshot_chop(&self, id: NodeId) -> Option<&ChopData> {
        self.ui_snapshot.chop.get(&id)
    }

    pub(crate) fn snapshot_node_message(&self, id: NodeId) -> Option<&NodeMessage> {
        self.ui_snapshot.node_messages.get(&id)
    }

    pub(crate) fn snapshot_avg_cook_ms(&self, id: NodeId) -> f64 {
        self.ui_snapshot
            .avg_cook_ms
            .get(&id)
            .copied()
            .unwrap_or(0.0)
    }

    pub(crate) fn snapshot_cook_rate(&self, id: NodeId) -> f64 {
        self.ui_snapshot.cook_rate.get(&id).copied().unwrap_or(0.0)
    }

    pub(crate) fn snapshot_has_cooked(&self, id: NodeId) -> bool {
        self.ui_snapshot.has_cooked.contains(&id)
    }

    pub(crate) fn snapshot_cooked_this_frame(&self, id: NodeId) -> bool {
        self.ui_snapshot.cooked_this_frame.contains(&id)
    }
}

fn refresh_snapshot(
    snap: &mut EngineUiSnapshot,
    rt: &ProjectRuntime,
    graph: &otd_core::Graph,
    frame: i64,
) {
    snap.frame = frame;
    snap.last_cook_us.clear();
    snap.time_dependent.clear();
    snap.output_version.clear();
    snap.top_meta.clear();
    snap.chop.clear();
    snap.node_messages.clear();
    snap.avg_cook_ms.clear();
    snap.cook_rate.clear();
    snap.has_cooked.clear();
    snap.cooked_this_frame.clear();

    let root = graph.root();
    for id in graph.walk() {
        if id == root {
            continue;
        }
        snap.last_cook_us.insert(id, rt.cook.last_cook_us(id));
        if rt.cook.is_time_dependent(id) {
            snap.time_dependent.insert(id);
        }
        snap.output_version.insert(id, rt.cook.output_version(id));
        snap.avg_cook_ms.insert(id, rt.cook.avg_cook_ms(id));
        snap.cook_rate.insert(id, rt.cook.cook_rate(id));
        if rt.cook.has_cooked(id) {
            snap.has_cooked.insert(id);
        }
        if rt.cook.cooked_this_frame(id, frame) {
            snap.cooked_this_frame.insert(id);
        }
        if let Some(tex) = rt.engines.top.output(graph, id) {
            snap.top_meta.insert(id, (tex.key.width, tex.key.height));
        }
        if graph.node(id).family == otd_core::Family::Chop {
            if let Some(data) = rt.engines.chop_data(id) {
                snap.chop.insert(id, data.clone());
            }
        }
        if let Some(msg) = rt.engines.node_message(graph, id) {
            snap.node_messages.insert(id, msg.clone());
        }
    }
}
