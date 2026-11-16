pub mod helpers;
#[allow(unused_imports)]
pub use helpers::*;

pub use otd_chop::ops;
#[allow(unused_imports)]
pub use otd_chop::{Channel, ChopData};
#[allow(unused_imports)]
pub use otd_core::param::{channel_expression, parameter_expression};
pub use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Value};
#[allow(unused_imports)]
pub use std::path::Path;

use otd_chop::ChopHost;

pub struct Patch {
    pub graph: Graph,
    pub reg: OpRegistry,
    pub host: ChopHost,
    pub cook: CookEngine,
    pub time: CookContext,
}

impl Patch {
    pub fn new() -> Self {
        Patch {
            graph: Graph::new(),
            reg: ops::registry(),
            host: ChopHost::new(),
            cook: CookEngine::new(),
            time: CookContext::default(),
        }
    }

    pub fn add(&mut self, op: &str, name: &str) -> NodeId {
        let root = self.graph.root();
        let def = self
            .reg
            .get(op)
            .unwrap_or_else(|| panic!("no operator `{op}`"))
            .clone();
        self.graph.create(root, &def, Some(name)).unwrap()
    }

    pub fn set(&mut self, id: NodeId, key: &str, value: Value) {
        self.graph.set_param(id, key, value).unwrap();
    }

    pub fn run(&mut self, root: NodeId, frames: usize) {
        for _ in 0..frames {
            self.cook
                .cook_frame(&self.graph, &[root], &self.time, &mut self.host)
                .unwrap();
            self.time.advance_nominal_frame();
        }
    }

    pub fn data(&self, id: NodeId) -> &ChopData {
        self.host.data(id).expect("node has cooked")
    }

    pub fn value(&self, id: NodeId, channel: &str) -> f32 {
        self.data(id)
            .value(channel)
            .unwrap_or_else(|| panic!("no channel `{channel}`; have {:?}", self.data(id).names()))
    }
}

impl Default for Patch {
    fn default() -> Self {
        Self::new()
    }
}
