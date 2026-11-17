//! The CHOP cook backend.
//!
//! The cooked channels live in a [`ChannelStore`] that is deliberately kept
//! *outside* the engine. Parameter expressions read from it while the
//! engine is mutating operator state, and splitting the two apart is what
//! lets both happen in the same frame without fighting the borrow checker —
//! or worse, without a lock in the cook path.

use std::sync::Arc;

use otd_core::{
    ChannelSource, CookContext, CookError, Cooker, Crossings, EvalContext, Family, Graph, NodeId,
    Value,
};
use slotmap::SecondaryMap;

use crate::data::ChopData;
use crate::io::{Io, PanelInteractionStore, is_panel_widget_op};
use crate::ops::{self, ChopCtx, OpState};

/// Every CHOP's most recent output.
#[derive(Default)]
pub struct ChannelStore {
    data: SecondaryMap<NodeId, Arc<ChopData>>,
}

impl ChannelStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, id: NodeId) -> Option<&ChopData> {
        self.data.get(id).map(|a| a.as_ref())
    }
    pub fn arc(&self, id: NodeId) -> Option<Arc<ChopData>> {
        self.data.get(id).cloned()
    }
    pub fn insert(&mut self, id: NodeId, data: Arc<ChopData>) {
        self.data.insert(id, data);
    }
    pub fn insert_owned(&mut self, id: NodeId, data: ChopData) {
        self.data.insert(id, Arc::new(data));
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
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// A read-only view of the network for parameter expressions.
pub struct Network<'a> {
    pub graph: &'a Graph,
    pub chops: &'a ChannelStore,
    pub panel: Option<&'a PanelInteractionStore>,
}

impl<'a> Network<'a> {
    pub fn new(graph: &'a Graph, chops: &'a ChannelStore) -> Self {
        Self {
            graph,
            chops,
            panel: None,
        }
    }

    pub fn with_panel(
        graph: &'a Graph,
        chops: &'a ChannelStore,
        panel: &'a PanelInteractionStore,
    ) -> Self {
        Self {
            graph,
            chops,
            panel: Some(panel),
        }
    }
}

impl ChannelSource for Network<'_> {
    fn channel_names(&self, op_path: &str) -> Vec<String> {
        self.graph
            .find(op_path)
            .and_then(|id| self.graph.resolve_output(id))
            .and_then(|id| self.chops.get(id))
            .map(|data| data.names())
            .unwrap_or_default()
    }

    fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
        // Resolving means a path can name a component and get the channels
        // its Out operator produces.
        let id = self.graph.resolve_output(self.graph.find(op_path)?)?;
        let data = self.chops.get(id)?;
        // By name first, then by index, so `chan1`/`0` both work and an
        // export survives a channel being renamed upstream if it keeps
        // its position.
        data.value(channel).or_else(|| {
            channel
                .parse::<usize>()
                .ok()
                .and_then(|i| data.nth(i))
                .map(|c| c.last())
        })
    }

    fn param_value(&self, op_path: &str, param: &str) -> Option<Value> {
        let id = self.graph.find(op_path)?;
        let node = self.graph.get(id)?;
        if let Some(panel) = self.panel {
            if is_panel_widget_op(&node.op_type) {
                if let Some(v) = panel.panel_param(op_path, param) {
                    return Some(Value::Float(v as f64));
                }
            }
        }
        let p = node.param_flexible(param)?;
        // Evaluated without a network of its own: `par()` reads a value without
        // chaining further parameter lookups.
        Some(p.eval(&EvalContext::default()))
    }

    fn parent_param(&self, node_path: &str, param: &str) -> Option<Value> {
        let id = self.graph.find(node_path)?;
        let parent_id = self.graph.get(id)?.parent?;
        let p = self.graph.get(parent_id)?.param_flexible(param)?;
        let parent_path = self.graph.path(parent_id);
        Some(p.eval(&EvalContext {
            channels: Some(self),
            path: Some(&parent_path),
            ..Default::default()
        }))
    }

    fn resolve_operator_path(&self, from_operator_path: &str, reference: &str) -> Option<String> {
        let reference = reference.trim();
        if reference.is_empty() {
            return None;
        }
        let from_id = self.graph.find(from_operator_path)?;
        let id = if reference.starts_with('/') {
            self.graph.find(reference)?
        } else {
            self.graph.find_from(from_id, reference)?
        };
        Some(self.graph.path(id))
    }
}

#[derive(Default)]
pub struct ChopEngine {
    state: SecondaryMap<NodeId, OpState>,
    pub io: Io,
    /// Feedback nodes cooked this frame, with the target to copy at `end_frame`.
    pending_feedback: Vec<(NodeId, NodeId)>,
    /// Target channels from the end of the previous frame.
    feedback_hold: SecondaryMap<NodeId, Arc<ChopData>>,
}

impl ChopEngine {
    pub fn new() -> Self {
        Self {
            state: Default::default(),
            io: Io::new(),
            pending_feedback: Vec::new(),
            feedback_hold: SecondaryMap::new(),
        }
    }

    /// Drop operator state and record buffers for ids no longer in the graph;
    /// recreate vacant slots for nodes an undo brought back (deleting bumps
    /// the secondary-map version).
    pub fn sync_with_graph(&mut self, graph: &Graph) {
        self.state.retain(|id, _| graph.contains(id));
        self.io.record_buffers.retain(|id, _| graph.contains(*id));
        self.feedback_hold.retain(|id, _| graph.contains(id));
        self.pending_feedback
            .retain(|(id, target)| graph.contains(*id) && graph.contains(*target));
        for (id, node) in graph.iter() {
            if node.family != Family::Chop {
                continue;
            }
            if !self.state.contains_key(id) {
                self.state.insert(id, OpState::default());
            }
        }
    }

    /// Cook one CHOP and return its channels. The caller stores them — see
    /// the module docs for why the engine does not.
    pub fn cook_node(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        eval: &EvalContext,
        chops: &ChannelStore,
        foreign: Crossings,
    ) -> Result<Arc<ChopData>, CookError> {
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let path = graph.path(id);

        // File parameters (Audio File In) resolve against the project's own
        // directory, the same rule external components follow.
        if self.io.base_dir.as_deref() != graph.base_dir() {
            self.io.base_dir = graph.base_dir().map(|p| p.to_path_buf());
        }

        // An In operator presents whatever is wired to its component from
        // outside; there is no operator to run.
        if node.connector == otd_core::Connector::In {
            return Ok(graph
                .connector_source(id)
                .and_then(|src| chops.arc(src))
                .unwrap_or_else(|| Arc::new(ChopData::empty())));
        }

        let spec = ops::spec_for(&node.op_type)
            .ok_or_else(|| CookError::op(&path, format!("unknown CHOP `{}`", node.op_type)))?;

        // Wires share [`Arc`] handles — cloning a static File In clip is cheap.
        let inputs: Vec<Arc<ChopData>> = node
            .inputs
            .iter()
            .enumerate()
            .map(|(i, slot)| {
                slot.and_then(|_| graph.resolve_input_source(id, i))
                    .and_then(|src| chops.arc(src))
                    .unwrap_or_else(|| Arc::new(ChopData::empty()))
            })
            .collect();

        // Parameters are evaluated knowing where they live, so `parent.x`
        // resolves against the right component.
        let eval = &EvalContext {
            path: Some(&path),
            ..*eval
        };
        if !self.state.contains_key(id) {
            self.state.insert(id, OpState::default());
        }
        let prev_frame = self.feedback_hold.get(id).cloned();
        let st = self.state.get_mut(id).ok_or(CookError::NoSuchNode)?;
        let io = &mut self.io;
        let mut cctx = ChopCtx {
            graph,
            id,
            node,
            eval,
            time: ctx,
            inputs,
            foreign,
            state: st,
            io,
            path: &path,
            store: chops,
            rate_override: None,
            prev_frame,
            capture: None,
            clear_prev: false,
            passthrough_input: None,
        };
        let feedback = node.op_type == "feedbackCHOP";
        let out = if node.op_type == crate::io::FILE_IN {
            crate::ops::cook_file_in_shared(&mut cctx)
        } else {
            let data = (spec.cook)(&mut cctx);
            if let Some(i) = cctx.passthrough_input {
                cctx.inputs
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| Arc::new(ChopData::empty()))
            } else {
                Arc::new(data)
            }
        };
        if feedback {
            let clear_prev = cctx.clear_prev;
            let capture = cctx.capture;
            drop(cctx);
            self.pending_feedback.retain(|(i, _)| *i != id);
            if clear_prev {
                self.feedback_hold.remove(id);
            } else if let Some(target) = capture {
                self.pending_feedback.push((id, target));
            }
        }
        Ok(out)
    }

    /// Copy each Feedback target's channels for the next frame.
    ///
    /// Done after the frame, not during the Feedback cook: a target upstream
    /// of the Feedback node has already cooked, and reading it then would
    /// drop the one-frame delay.
    pub fn end_frame(&mut self, channels: &ChannelStore) {
        for (id, target) in std::mem::take(&mut self.pending_feedback) {
            if let Some(data) = channels.arc(target) {
                self.feedback_hold.insert(id, data);
            }
        }
    }

    /// A device or network message for this node, shown on the node body.
    pub fn status(&self, path: &str) -> Option<&str> {
        self.io.status_of(path)
    }

    pub fn forget(&mut self, id: NodeId) {
        self.state.remove(id);
        self.io.record_buffers.remove(&id);
        self.feedback_hold.remove(id);
        self.pending_feedback.retain(|(i, _)| *i != id);
    }

    pub fn reset(&mut self) {
        self.state.clear();
        self.io.reset();
        self.feedback_hold.clear();
        self.pending_feedback.clear();
    }
}

/// A CHOP-only host: the engine plus its store, implementing [`Cooker`].
///
/// The editor uses the cross-family router in `otd-engine` instead; this
/// exists for tests and for a headless control-only runtime.
#[derive(Default)]
pub struct ChopHost {
    pub engine: ChopEngine,
    pub channels: ChannelStore,
}

impl ChopHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn data(&self, id: NodeId) -> Option<&ChopData> {
        self.channels.get(id)
    }

    /// The current value of a channel, by node and channel name.
    pub fn value(&self, graph: &Graph, path: &str, channel: &str) -> Option<f32> {
        Network::new(graph, &self.channels).channel(path, channel)
    }

    pub fn end_frame(&mut self) {
        let ChopHost { engine, channels } = self;
        engine.end_frame(channels);
    }
}

impl Cooker for ChopHost {
    fn cook(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
    ) -> Result<otd_core::CookOutput, CookError> {
        if graph.get(id).map(|n| n.family) != Some(Family::Chop) {
            return Ok(otd_core::CookOutput::Updated);
        }
        let ChopHost { engine, channels } = self;
        let data = {
            let net = Network::new(graph, channels);
            let path = graph.path(id);
            let eval = ctx.eval_for_node(graph, id, &net, &path);
            engine.cook_node(graph, id, ctx, &eval, channels, Crossings::new())?
        };
        channels.insert(id, data);
        Ok(otd_core::CookOutput::Updated)
    }
}
