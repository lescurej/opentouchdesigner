//! Graph traversal helpers.

use slotmap::{Key, SecondaryMap};

use super::{Graph, NodeId};

impl Graph {
    /// Depth-first walk of the whole hierarchy, parents before children.
    pub fn walk(&self) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack = vec![self.root];
        while let Some(cur) = stack.pop() {
            out.push(cur);
            for c in self.nodes[cur].children.iter().rev() {
                stack.push(*c);
            }
        }
        out
    }

    /// Every operator in the project — cook roots that do not depend on UI visibility.
    pub fn all_operator_roots(&self) -> Vec<NodeId> {
        self.walk()
            .into_iter()
            .filter(|id| *id != self.root())
            .collect()
    }

    /// Build a `SecondaryMap` sized for this graph. Used by the cook engine
    /// and by the GPU crate to hang per-node state off node ids.
    pub fn secondary<T>(&self) -> SecondaryMap<NodeId, T> {
        SecondaryMap::new()
    }

    /// Debug helper — a stable, human-readable id for logging.
    pub fn debug_id(&self, id: NodeId) -> String {
        if self.nodes.contains_key(id) {
            self.path(id)
        } else {
            format!("<dead {:?}>", id.data())
        }
    }
}
