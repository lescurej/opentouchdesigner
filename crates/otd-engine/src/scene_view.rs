//! Scene trait for TOP cooking.

use otd_chop::engine::ChannelStore;
use otd_core::Graph;
use otd_sop::{Geometry, GeometryStore};

pub(crate) struct SceneView<'a> {
    pub(crate) graph: &'a Graph,
    pub(crate) geometry: &'a GeometryStore,
    pub(crate) channels: &'a ChannelStore,
}

impl otd_gpu::scene::Scene for SceneView<'_> {
    fn geometry(&self, path: &str) -> Option<&Geometry> {
        // Resolving means a path can name a component and get the geometry
        // its Out operator produces.
        let id = self.graph.resolve_output(self.graph.find(path)?)?;
        self.geometry.get(id)
    }

    fn channels(&self, path: &str) -> Option<Vec<(String, Vec<f32>)>> {
        let id = self.graph.resolve_output(self.graph.find(path)?)?;
        let data = self.channels.get(id)?;
        Some(
            data.channels
                .iter()
                .map(|c| (c.name.clone(), c.samples.clone()))
                .collect(),
        )
    }
}

