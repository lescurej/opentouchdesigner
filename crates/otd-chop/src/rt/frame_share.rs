use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use otd_core::{CookContext, Graph};

use crate::engine::ChannelStore;
use crate::io::Io;

/// What the frame thread hands to the audio worker each frame. The worker
/// holds each lock only long enough to clone an `Arc` or copy a struct, and
/// the graph is republished only when its content changes.
pub struct FrameShare {
    graph: Mutex<Arc<Graph>>,
    channels: Mutex<Arc<ChannelStore>>,
    time: Mutex<CookContext>,
    io: Mutex<Io>,
    io_generation: AtomicU64,
}

impl FrameShare {
    pub fn new(graph: Arc<Graph>) -> Arc<Self> {
        Arc::new(Self {
            graph: Mutex::new(graph),
            channels: Mutex::new(Arc::new(ChannelStore::default())),
            time: Mutex::new(CookContext::default()),
            io: Mutex::new(Io::default()),
            io_generation: AtomicU64::new(0),
        })
    }

    pub fn publish_graph(&self, graph: Arc<Graph>) {
        if let Ok(mut slot) = self.graph.lock() {
            *slot = graph;
        }
    }

    /// The frame cook's channels, for parameter expressions and operators
    /// that name another CHOP.
    pub fn publish_channels(&self, channels: Arc<ChannelStore>) {
        if let Ok(mut slot) = self.channels.lock() {
            *slot = channels;
        }
    }

    pub fn publish_time(&self, time: &CookContext) {
        if let Ok(mut slot) = self.time.lock() {
            *slot = *time;
        }
    }

    pub fn publish_io(&self, io: &Io) {
        let snapshot = io.snapshot_for_rt_cook();
        if let Ok(mut slot) = self.io.lock() {
            *slot = snapshot;
            self.io_generation.fetch_add(1, Ordering::Release);
        }
    }

    pub(crate) fn graph(&self) -> Option<Arc<Graph>> {
        self.graph.lock().ok().map(|g| g.clone())
    }

    pub(crate) fn channels(&self) -> Arc<ChannelStore> {
        self.channels.lock().map(|c| c.clone()).unwrap_or_default()
    }

    pub(crate) fn time(&self) -> CookContext {
        self.time.lock().map(|t| *t).unwrap_or_default()
    }

    pub(crate) fn io_generation(&self) -> u64 {
        self.io_generation.load(Ordering::Acquire)
    }

    pub(crate) fn refresh_io(&self, local: &mut Io) {
        if let Ok(io) = self.io.lock() {
            *local = io.snapshot_for_rt_cook();
        }
    }
}
