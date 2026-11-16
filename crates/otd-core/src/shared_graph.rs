use std::sync::{Arc, RwLock};

use crate::Graph;

#[derive(Clone)]
pub struct SharedGraph {
    inner: Arc<RwLock<Graph>>,
    generation: Arc<std::sync::atomic::AtomicU64>,
}

impl SharedGraph {
    pub fn new(graph: Graph) -> Self {
        let topology = graph.audio_topology_generation();
        Self {
            inner: Arc::new(RwLock::new(graph)),
            generation: Arc::new(std::sync::atomic::AtomicU64::new(topology)),
        }
    }

    pub fn from_arc(inner: Arc<RwLock<Graph>>) -> Self {
        let topology = inner
            .read()
            .map(|g| g.audio_topology_generation())
            .unwrap_or(0);
        Self {
            inner,
            generation: Arc::new(std::sync::atomic::AtomicU64::new(topology)),
        }
    }

    pub fn arc(&self) -> &Arc<RwLock<Graph>> {
        &self.inner
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn refresh_generation(&self) {
        let Ok(g) = self.inner.read() else {
            return;
        };
        self.generation
            .store(g.audio_topology_generation(), std::sync::atomic::Ordering::Release);
    }

    pub fn read<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Graph) -> R,
    {
        self.inner.read().ok().map(|g| f(&g))
    }

    pub fn write<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Graph) -> R,
    {
        let out = self.inner.write().ok().map(|mut g| f(&mut g));
        self.refresh_generation();
        out
    }
}