//! Undo and redo.
//!
//! PLAN.md Phase 6 asks for "undo/redo everywhere", and *everywhere* is the
//! hard part. The usual approach — one invertible command per operation — has
//! to be got right for creating, deleting, wiring, unwiring, every parameter
//! mode, custom parameters, component attachment and clone syncing, and every
//! one of those is a separate chance to write an inverse that is subtly wrong.
//! An undo that *mostly* works is worse than none, because you only find out
//! after you have lost the work.
//!
//! So this snapshots the graph instead. A graph is plain data — nodes, params,
//! wires — and cloning it costs on the order of the project size, paid only
//! when something is edited rather than per frame. The snapshot preserves node
//! ids, which is what makes it usable: the selection, the viewer and the cook
//! engine's caches all still refer to the same nodes after an undo.
//!
//! The one thing snapshots need on top is **coalescing**. Dragging a slider
//! emits an edit every frame, and sixty undo entries for one gesture is not
//! undo. Each checkpoint carries a tag naming what is being edited; while the
//! tag stays the same, the gesture is one entry.

use crate::graph::Graph;

/// How many steps back you can go. Fifty is enough to cover a session's worth
/// of "what did I just break", and the memory is a graph clone apiece.
pub const DEFAULT_LIMIT: usize = 50;

pub struct History {
    past: Vec<Graph>,
    future: Vec<Graph>,
    limit: usize,
    /// What the last checkpoint was editing, for coalescing.
    last_tag: Option<String>,
}

impl Default for History {
    fn default() -> Self {
        History::new(DEFAULT_LIMIT)
    }
}

impl History {
    pub fn new(limit: usize) -> History {
        History {
            past: Vec::new(),
            future: Vec::new(),
            limit,
            last_tag: None,
        }
    }

    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        if limit == 0 {
            self.past.clear();
            self.future.clear();
            self.last_tag = None;
            return;
        }
        while self.past.len() > limit {
            self.past.remove(0);
        }
    }

    /// Record the state *before* a change.
    ///
    /// `tag` names what is about to be edited — `"param:/level1/brightness"`,
    /// `"move:/noise1"`. Consecutive checkpoints with the same tag are one
    /// entry, so a drag undoes as the gesture it was rather than as the sixty
    /// frames it took.
    pub fn checkpoint(&mut self, graph: &Graph, tag: &str) {
        if self.limit == 0 {
            return;
        }
        if self.last_tag.as_deref() == Some(tag) && !self.past.is_empty() {
            // Mid-gesture: the entry already holds the state before it began,
            // which is exactly where undo should land.
            self.future.clear();
            return;
        }
        self.last_tag = Some(tag.to_string());
        self.past.push(graph.clone());
        if self.past.len() > self.limit {
            self.past.remove(0);
        }
        // A new edit is a new branch: whatever was undone is no longer ahead.
        self.future.clear();
    }

    /// End the current gesture, so the next edit starts a new undo entry even
    /// if it carries the same tag. Called on mouse-up and on focus loss.
    pub fn end_gesture(&mut self) {
        self.last_tag = None;
    }

    /// Step back. `current` is the state being left, which becomes the redo.
    pub fn undo(&mut self, current: &Graph) -> Option<Graph> {
        let previous = self.past.pop()?;
        self.future.push(current.clone());
        self.last_tag = None;
        Some(previous)
    }

    pub fn redo(&mut self, current: &Graph) -> Option<Graph> {
        let next = self.future.pop()?;
        self.past.push(current.clone());
        self.last_tag = None;
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Forget everything — opening a project, or starting a new one.
    pub fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
        self.last_tag = None;
    }

    pub fn depth(&self) -> (usize, usize) {
        (self.past.len(), self.future.len())
    }
}

#[cfg(test)]
#[path = "tests/history__tests.rs"]
mod tests;
