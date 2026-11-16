use otd_core::{ChannelSource, CookContext, CookError, Cooker, Crossings, Family, Graph, NodeId};

use super::data::DatData;
use super::engine::DatEngine;
use super::store::DatStore;

/// A DAT-only host, for tests and for a control-only runtime.
#[derive(Default)]
pub struct DatHost {
    pub engine: DatEngine,
    pub store: DatStore,
}

impl DatHost {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn data(&self, id: NodeId) -> Option<&DatData> {
        self.store.get(id)
    }
}

impl Cooker for DatHost {
    fn cook(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
    ) -> Result<otd_core::CookOutput, CookError> {
        if graph.get(id).map(|n| n.family) != Some(Family::Dat) {
            return Ok(otd_core::CookOutput::Updated);
        }
        let DatHost { engine, store } = self;
        let eval = ctx.eval_ctx();
        let data = engine.cook_node(
            graph,
            id,
            ctx,
            &eval,
            store,
            None,
            Crossings::new(),
            None,
            None,
        )?;
        store.insert(id, data);
        Ok(otd_core::CookOutput::Updated)
    }
}

/// A read-only view of DAT contents, so a parameter can read a table cell.
pub struct DatNetwork<'a> {
    pub graph: &'a Graph,
    pub dats: &'a DatStore,
}

impl ChannelSource for DatNetwork<'_> {
    fn channel(&self, _op_path: &str, _channel: &str) -> Option<f32> {
        None
    }
    fn param_value(&self, _op_path: &str, _param: &str) -> Option<otd_core::Value> {
        None
    }
}
