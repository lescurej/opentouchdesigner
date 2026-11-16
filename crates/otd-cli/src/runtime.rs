//! Running a project without a window — delegates to `otd_engine::ProjectRuntime`.

use std::ops::{Deref, DerefMut};
use std::path::Path;

use otd_core::NodeId;
use otd_engine::{
    FrameConfig, FrameTiming, OutputRootsRequest, PlaybackState, ProjectRuntime, output_roots,
};

pub use otd_engine::summarize;

pub struct Runtime(pub ProjectRuntime);

impl Deref for Runtime {
    type Target = ProjectRuntime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Runtime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Runtime {
    pub fn open(path: &Path) -> Result<Runtime, String> {
        ProjectRuntime::open(path).map(Runtime)
    }

    pub fn roots(&self) -> Vec<NodeId> {
        output_roots(OutputRootsRequest {
            graph: &self.graph,
            perform_ui: None,
            extra_roots: &[],
        })
    }

    pub fn find(&self, path: &str) -> Result<NodeId, String> {
        self.graph
            .find(path)
            .ok_or_else(|| format!("no node at `{path}`"))
    }

    pub fn frame(&mut self, roots: &[NodeId], dt: f64) -> Result<FrameTiming, String> {
        let mut playback = PlaybackState {
            playing: true,
            ..Default::default()
        };
        let result = self.tick_frame(
            FrameConfig {
                dt,
                roots: roots.to_vec(),
                audio_latency_ms: 0,
                input: Default::default(),
                display_blits: Vec::new(),
                defer_gpu_submit: false,
                max_time_slice_ms: otd_core::project::DEFAULT_MAX_TIME_SLICE_MS,
                advance_timeline: true,
            },
            &mut playback,
        );
        result
            .cook_error
            .map(Err)
            .unwrap_or(Ok(result.timing))
    }

    pub fn read(&self, id: NodeId) -> Result<(u32, u32, Vec<u8>), String> {
        self.read_top_rgba8(id)
    }

    pub fn shader_errors(&self) -> Vec<String> {
        self.graph
            .walk()
            .into_iter()
            .filter_map(|id| {
                self.engines
                    .top
                    .shader_error(id)
                    .map(|e| format!("{}: {e}", self.graph.path(id)))
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "tests/runtime__tests.rs"]
mod tests;
