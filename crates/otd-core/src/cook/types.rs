use crate::graph::{Graph, NodeId};

use super::context::CookContext;

#[derive(Debug, thiserror::Error)]
pub enum CookError {
    #[error("{path}: {message}")]
    Op { path: String, message: String },
    #[error("node vanished mid-cook")]
    NoSuchNode,
}

impl CookError {
    pub fn op(path: impl Into<String>, message: impl Into<String>) -> Self {
        CookError::Op {
            path: path.into(),
            message: message.into(),
        }
    }
}

/// Whether a cook produced new output for downstream nodes to react to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CookOutput {
    Updated,
    Unchanged,
}

/// Implemented by whatever actually produces operator output.
pub trait Cooker {
    /// Cook one node. Its inputs are guaranteed to have been cooked already
    /// this frame.
    fn cook(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
    ) -> Result<CookOutput, CookError>;

    /// Called when a node is skipped because its cached output is still
    /// valid. Default: nothing.
    fn skipped(&mut self, _graph: &Graph, _id: NodeId) {}

    /// Dependencies a node has that are not wires.
    ///
    /// Some operators name their source by parameter rather than by input —
    /// a Select TOP pointing at `/blur1`, and later a parameter expression
    /// referencing another operator. Those still have to cook first and still
    /// have to propagate dirtiness and time dependence, so the backend
    /// declares them here and the engine treats them exactly like inputs.
    ///
    /// A Feedback TOP deliberately does *not* declare its target: reading
    /// last frame's result is the whole point, and declaring it would be a
    /// cycle.
    fn extra_inputs(&self, _graph: &Graph, _id: NodeId) -> Vec<NodeId> {
        Vec::new()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FrameStats {
    /// Nodes that actually ran their operator this frame.
    pub cooked: u32,
    /// Nodes that were reached but served from cache.
    pub cached: u32,
    pub total_cook_us: u64,
}
