use slotmap::SecondaryMap;
use std::time::Instant;

use crate::graph::{Graph, NodeId};

use super::context::CookContext;
use super::types::{CookError, CookOutput, Cooker, FrameStats};

#[derive(Clone, Debug, Default)]
pub(crate) struct NodeCookState {
    /// The graph revision this node was cooked at.
    revision: u64,
    /// Output versions of each input at the last cook.
    input_versions: Vec<u64>,
    /// Bumped every time the node actually cooks. Downstream nodes compare
    /// against it to decide whether their input changed.
    output_version: u64,
    /// `CookEngine::pass` when this node was last resolved. Diamonds in one
    /// `cook_frame` share a pass; the next repaint gets a new one even when
    /// the timeline frame number did not move.
    resolved_pass: u64,
    time_dependent: bool,
    cook_count: u64,
    last_cook_us: u64,
    /// Exponential moving average of the cook time, in microseconds.
    ///
    /// A single frame's measurement is noise — a GC pause, a scheduler hiccup,
    /// the first compile. A performance monitor built on the raw number is a
    /// flicker of digits nobody can read, let alone rank by.
    avg_cook_us: f64,
    /// Frames this node was reached at all, cooked or cached. With
    /// `cook_count` this gives how *often* it actually runs, which is the
    /// other half of what a node costs: something expensive that cooks once a
    /// second is cheaper than something modest that cooks every frame.
    resolve_count: u64,
    has_cooked: bool,
    /// Enclosing component revision when this node reads `parent.*` params.
    parent_revision: u64,
    /// Frame number when the operator last ran (not a cache hit).
    last_cook_frame: i64,
    /// Set while this node is on the pull stack. Wired cycles are impossible
    /// (rejected at connect time) but a parameter-named reference can form
    /// one — two Select TOPs pointing at each other — and that must degrade
    /// to a stale read, not a stack overflow.
    visiting: bool,
}

#[derive(Default)]
pub struct CookEngine {
    state: SecondaryMap<NodeId, NodeCookState>,
    pub stats: FrameStats,
    /// Bumped at the start of every `cook_frame`. See `resolved_pass`.
    pass: u64,
}

/// Clears [`NodeCookState::visiting`] when a pull returns early on error.
impl CookEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn state_mut(&mut self, id: NodeId) -> Result<&mut NodeCookState, CookError> {
        if !self.state.contains_key(id) {
            self.state.insert(id, NodeCookState::default());
        }
        self.state.get_mut(id).ok_or(CookError::NoSuchNode)
    }

    /// Cook everything the given roots depend on, for one frame.
    pub fn cook_frame(
        &mut self,
        graph: &Graph,
        roots: &[NodeId],
        ctx: &CookContext,
        cooker: &mut dyn Cooker,
    ) -> Result<(), CookError> {
        self.pass = self.pass.wrapping_add(1);
        self.stats = FrameStats::default();
        self.clear_visiting();
        for root in roots {
            if !graph.contains(*root) {
                continue;
            }
            self.pull(graph, *root, ctx, cooker)?;
        }
        Ok(())
    }

    /// Clear the pull-stack flag on every node. Call after a failed cook or a
    /// graph swap so the engine cannot get stuck serving stale cache forever.
    pub fn clear_visiting(&mut self) {
        for st in self.state.values_mut() {
            st.visiting = false;
        }
    }

    #[cfg(test)]
    pub(crate) fn test_set_visiting(
        &mut self,
        id: NodeId,
        visiting: bool,
    ) -> Result<(), CookError> {
        self.state_mut(id)?.visiting = visiting;
        Ok(())
    }

    /// Drop cook state for operators no longer in the graph. Node ids are stable
    /// across undo, but an entry can outlive a deleted id until the slot reuses.
    pub fn retain_graph(&mut self, graph: &Graph) {
        self.state.retain(|id, _| graph.contains(id));
    }

    /// Keep cook caches consistent after undo/redo or any wholesale graph swap.
    pub fn sync_with_graph(&mut self, graph: &Graph) {
        self.clear_visiting();
        self.retain_graph(graph);
        for (id, _) in graph.iter() {
            if !self.state.contains_key(id) {
                self.state.insert(id, NodeCookState::default());
            }
        }
    }

    /// Resolve one node for this frame, cooking it and its dependencies only
    /// as far as necessary. Returns the node's output version.
    pub fn pull(
        &mut self,
        graph: &Graph,
        id: NodeId,
        ctx: &CookContext,
        cooker: &mut dyn Cooker,
    ) -> Result<u64, CookError> {
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let pass = self.pass;

        // Already on the stack via a parameter-named reference cycle.
        if let Some(st) = self.state.get(id) {
            if st.visiting {
                return Ok(st.output_version);
            }
            // Diamond inputs pull the same ancestor twice in one cook_frame.
            if st.has_cooked
                && st.resolved_pass == pass
                && st.last_cook_frame == ctx.frame
            {
                let edited = st.revision != node.revision();
                let parent_revision = if node.references_parent() {
                    node.parent
                        .and_then(|p| graph.get(p))
                        .map(|p| p.revision())
                        .unwrap_or(0)
                } else {
                    0
                };
                let parent_changed =
                    node.references_parent() && st.parent_revision != parent_revision;
                if !edited && !parent_changed {
                    return Ok(st.output_version);
                }
            }
        }
        {
            let st = self.state_mut(id)?;
            st.resolved_pass = pass;
            st.resolve_count += 1;
            st.visiting = true;
        }

        // Pull inputs first, collecting their versions and time dependence.
        let mut input_versions = Vec::with_capacity(node.inputs.len());
        let mut inputs_time_dependent = false;
        for slot in &node.inputs {
            match slot {
                Some(src) if graph.contains(*src) => {
                    let v = self.pull(graph, *src, ctx, cooker)?;
                    inputs_time_dependent |= self
                        .state
                        .get(*src)
                        .map(|s| s.time_dependent)
                        .unwrap_or(false);
                    input_versions.push(v);
                }
                // An unconnected input is version 0 — stable, so it never
                // forces a re-cook on its own.
                None | Some(_) => input_versions.push(0),
            }
        }

        // Parameters in Expression mode read from other operators via `ch()`,
        // `par()`, etc. Those operators cook first, and their animation
        // reaches this node, so they are dependencies in every sense except
        // the wire.
        let mut param_sources: Vec<NodeId> = node
            .param_sources()
            .filter_map(|path| graph.find_from(id, path))
            .collect();

        // Component boundaries are dependencies too, in both directions: an
        // In operator depends on whatever is wired to the component from
        // outside, and a component depends on the Out operator inside it.
        // Expressing them here means encapsulation costs the cook engine
        // nothing — dirtiness and time dependence cross the boundary by the
        // same rules as a wire.
        if node.connector == crate::graph::Connector::In {
            param_sources.extend(graph.connector_source(id));
        }
        if node.family == crate::graph::Family::Comp {
            param_sources.extend(graph.connector_ops(id, crate::graph::Connector::Out));
        }

        // An operator reading `parent.speed` inherits its component's
        // dependencies — but not the component itself, which would be a cycle
        // (the component already depends on the Out operator inside it).
        let mut parent_animated = false;
        if node.references_parent() {
            if let Some(parent) = node.parent.and_then(|p| graph.get(p)) {
                parent_animated = parent.has_time_dependent_param();
                param_sources.extend(
                    parent
                        .param_sources()
                        .filter_map(|p| graph.find_from(id, p)),
                );
            }
        }

        // Non-wire dependencies (a Select TOP's target, a parameter expression
        // source) are pulled and versioned exactly like wired inputs, so they
        // dirty and animate this node the same way.
        for src in param_sources
            .into_iter()
            .chain(cooker.extra_inputs(graph, id))
        {
            if src == id || !graph.contains(src) {
                continue;
            }
            let v = self.pull(graph, src, ctx, cooker)?;
            inputs_time_dependent |= self
                .state
                .get(src)
                .map(|s| s.time_dependent)
                .unwrap_or(false);
            input_versions.push(v);
        }

        let self_time_dependent =
            node.intrinsically_time_dependent || node.has_time_dependent_param() || parent_animated;
        let time_dependent = self_time_dependent || inputs_time_dependent;

        // Every recursive pull for this node is done; it is no longer on the
        // stack.
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let needs_cook = {
            let st = self.state_mut(id)?;
            st.visiting = false;
            let inputs_changed = st.input_versions != input_versions;
            let edited = st.revision != node.revision();
            let parent_revision = if node.references_parent() {
                node.parent
                    .and_then(|p| graph.get(p))
                    .map(|p| p.revision())
                    .unwrap_or(0)
            } else {
                0
            };
            let parent_changed = node.references_parent() && st.parent_revision != parent_revision;
            st.time_dependent = time_dependent;
            st.input_versions = input_versions;
            st.revision = node.revision();
            st.parent_revision = parent_revision;
            !st.has_cooked || edited || inputs_changed || parent_changed || self_time_dependent
        };

        if !needs_cook {
            self.stats.cached += 1;
            cooker.skipped(graph, id);
            return Ok(self.state.get(id).map(|s| s.output_version).unwrap_or(0));
        }

        // Bypassed nodes pass input 0 straight through without running.
        if node.flags.bypass {
            let passthrough = node.inputs.first().and_then(|s| *s);
            let version = passthrough
                .and_then(|src| self.state.get(src).map(|s| s.output_version))
                .unwrap_or(0);
            {
                let st = self.state_mut(id)?;
                st.output_version = version;
                st.has_cooked = true;
            }
            self.stats.cached += 1;
            return Ok(version);
        }

        let started = Instant::now();
        let output = cooker.cook(graph, id, ctx)?;
        let elapsed = started.elapsed().as_micros() as u64;

        let (version, counted_cache) = {
            let st = self.state_mut(id)?;
            let counted_cache = output != CookOutput::Updated;
            if output == CookOutput::Updated {
                st.output_version += 1;
            }
            st.cook_count += 1;
            st.last_cook_us = elapsed;
            // Weighted toward history: the point is to be readable and rankable,
            // not to chase the last frame.
            st.avg_cook_us = if st.cook_count <= 1 {
                elapsed as f64
            } else {
                st.avg_cook_us * 0.9 + elapsed as f64 * 0.1
            };
            st.has_cooked = true;
            st.last_cook_frame = ctx.frame;
            (st.output_version, counted_cache)
        };
        if counted_cache {
            self.stats.cached += 1;
        }
        self.stats.cooked += 1;
        self.stats.total_cook_us += elapsed;
        Ok(version)
    }

    /// True when this node's operator actually ran on this frame (not served from cache).
    pub fn cooked_this_frame(&self, id: NodeId, frame: i64) -> bool {
        self.state
            .get(id)
            .is_some_and(|s| s.last_cook_frame == frame)
    }

    /// Forget all cached state. Call after loading a project.
    pub fn reset(&mut self) {
        self.state.clear();
        self.stats = FrameStats::default();
    }

    pub fn forget(&mut self, id: NodeId) {
        self.state.remove(id);
    }

    /// How many times this node has actually run its operator.
    pub fn cook_count(&self, id: NodeId) -> u64 {
        self.state.get(id).map(|s| s.cook_count).unwrap_or(0)
    }

    /// Microseconds spent in the most recent cook — the per-node number the
    /// performance monitor shows (PLAN.md §5, Phase 6).
    pub fn last_cook_us(&self, id: NodeId) -> u64 {
        self.state.get(id).map(|s| s.last_cook_us).unwrap_or(0)
    }

    /// Smoothed cook time in milliseconds — what this node costs when it runs.
    pub fn avg_cook_ms(&self, id: NodeId) -> f64 {
        self.state.get(id).map(|s| s.avg_cook_us).unwrap_or(0.0) / 1000.0
    }

    /// Fraction of the frames this node was reached on which it actually
    /// cooked, 0..1.
    ///
    /// The other half of what something costs. A node that takes 4 ms but only
    /// cooks when a parameter changes is not the reason a patch is dropping
    /// frames; one that takes 0.6 ms every single frame might be.
    pub fn cook_rate(&self, id: NodeId) -> f64 {
        match self.state.get(id) {
            Some(s) if s.resolve_count > 0 => s.cook_count as f64 / s.resolve_count as f64,
            _ => 0.0,
        }
    }

    /// Smoothed cost *per frame*: what this node adds to a typical frame,
    /// which is what a performance list should rank by.
    pub fn frame_cost_ms(&self, id: NodeId) -> f64 {
        self.avg_cook_ms(id) * self.cook_rate(id)
    }

    pub fn is_time_dependent(&self, id: NodeId) -> bool {
        self.state
            .get(id)
            .map(|s| s.time_dependent)
            .unwrap_or(false)
    }

    pub fn output_version(&self, id: NodeId) -> u64 {
        self.state.get(id).map(|s| s.output_version).unwrap_or(0)
    }

    pub fn has_cooked(&self, id: NodeId) -> bool {
        self.state.get(id).map(|s| s.has_cooked).unwrap_or(false)
    }
}
