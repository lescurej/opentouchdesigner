use otd_core::graph::Graph;
use otd_core::NodeId;
use slotmap::SecondaryMap;

use super::data::DatData;

/// Every DAT's most recent output.
#[derive(Default)]
pub struct DatStore {
    data: SecondaryMap<NodeId, DatData>,
}

impl DatStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, id: NodeId) -> Option<&DatData> {
        self.data.get(id)
    }
    pub fn insert(&mut self, id: NodeId, data: DatData) {
        self.data.insert(id, data);
    }
    pub fn remove(&mut self, id: NodeId) {
        self.data.remove(id);
    }
    pub fn retain_graph(&mut self, graph: &Graph) {
        self.data.retain(|id, _| graph.contains(id));
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
}
