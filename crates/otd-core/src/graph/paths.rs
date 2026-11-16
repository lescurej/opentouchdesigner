//! Paths, references, and expression links.

use std::collections::HashSet;

use crate::channel_pattern::{operator_reference_is_pattern, pattern_matches_operator};
use crate::param::{Param, ParamMode};

use super::{Family, Graph, NodeId};

impl Graph {
    // --------------------------------------------------------------- paths

    /// `/geo1/noise1` — the address artists and scripts use.
    pub fn path(&self, id: NodeId) -> String {
        let mut parts = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            if c == self.root {
                break;
            }
            let n = &self.nodes[c];
            parts.push(n.name.clone());
            cur = n.parent;
        }
        parts.reverse();
        format!("/{}", parts.join("/"))
    }

    pub fn relative_op_path(&self, referencer: NodeId, target: NodeId) -> Option<String> {
        if referencer == target {
            return None;
        }
        let anchor = self.node(referencer).parent.unwrap_or(self.root);
        if self.is_descendant_of(anchor, target) {
            return Some(self.path_from_ancestor(anchor, target));
        }
        let mut ups = 0u32;
        let mut cur = anchor;
        loop {
            if self.is_descendant_of(cur, target) {
                let down = self.path_from_ancestor(cur, target);
                let prefix = (0..ups).map(|_| "..").collect::<Vec<_>>().join("/");
                return Some(if prefix.is_empty() {
                    down
                } else if down.is_empty() {
                    prefix
                } else {
                    format!("{prefix}/{down}")
                });
            }
            if cur == self.root {
                return Some(self.path(target));
            }
            cur = self.node(cur).parent.unwrap_or(self.root);
            ups += 1;
        }
    }

    fn is_descendant_of(&self, ancestor: NodeId, node: NodeId) -> bool {
        let mut cur = Some(node);
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            cur = self.node(c).parent;
        }
        false
    }

    fn path_from_ancestor(&self, ancestor: NodeId, node: NodeId) -> String {
        let mut names = Vec::new();
        let mut cur = node;
        while cur != ancestor {
            names.push(self.node(cur).name.clone());
            cur = self.node(cur).parent.expect("ancestor chain");
        }
        names.reverse();
        names.join("/")
    }

    pub fn local_clock_at(&self, node: NodeId, global: &crate::cook::CookContext) -> (f64, f64, i64) {
        let mut time = global.time;
        let mut abs_time = global.abs_time;
        let mut frame = global.frame;
        let mut cur = node;
        while let Some(n) = self.get(cur) {
            let Some(parent) = n.parent else {
                break;
            };
            let comp = &self.nodes[parent];
            if comp.family == Family::Comp {
                let start = comp
                    .param("timestart")
                    .map(|p| p.value.as_f64())
                    .unwrap_or(0.0);
                let rate = comp.param("rate").map(|p| p.value.as_f64()).unwrap_or(1.0);
                time = (time - start).max(0.0) * rate;
                abs_time = (abs_time - start).max(0.0) * rate;
                frame = ((frame as f64 - start * global.fps) * rate).round() as i64;
            }
            cur = parent;
        }
        (time, abs_time, frame)
    }

    pub fn find(&self, path: &str) -> Option<NodeId> {
        let mut cur = self.root;
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            let next = self.nodes[cur]
                .children
                .iter()
                .find(|c| self.nodes[**c].name == seg)?;
            cur = *next;
        }
        Some(cur)
    }

    /// CHOP channel links via Export mode or expressions (`ch()`, `op()` on a CHOP).
    pub fn channel_expression_links(&self) -> Vec<(NodeId, NodeId)> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for dst in self.walk() {
            if dst == self.root() {
                continue;
            }
            for p in self.node(dst).params.values() {
                let sources: Vec<NodeId> = match p.mode {
                    ParamMode::Export => self
                        .resolve_param_op_path(dst, p)
                        .into_iter()
                        .collect(),
                    ParamMode::Expression => self.expression_chop_sources(dst, p),
                    _ => Vec::new(),
                };
                for src in sources {
                    if src == dst || !self.contains(src) {
                        continue;
                    }
                    if seen.insert((src, dst)) {
                        out.push((src, dst));
                    }
                }
            }
        }
        out
    }

    fn resolve_param_op_path(&self, from: NodeId, p: &Param) -> Option<NodeId> {
        let path = p.source_parts().map(|(path, _)| path)?;
        self.resolve_ref_path(from, path)
    }

    fn expression_chop_sources(&self, from: NodeId, p: &Param) -> Vec<NodeId> {
        p.referenced_ops()
            .filter_map(|path| self.resolve_ref_path(from, path))
            .filter(|&id| self.nodes.get(id).is_some_and(|n| n.family == Family::Chop))
            .collect()
    }

    pub fn chops_matching_reference(&self, from: NodeId, reference: &str) -> Vec<NodeId> {
        let reference = reference.trim();
        if reference.is_empty() {
            return Vec::new();
        }
        if operator_reference_is_pattern(reference) {
            let mut walk = from;
            let mut scope = self.root;
            while let Some(parent) = self.nodes[walk].parent {
                if self.nodes[parent].family == Family::Comp {
                    scope = parent;
                }
                walk = parent;
            }
            let mut matches = Vec::new();
            let mut stack = vec![scope];
            while let Some(id) = stack.pop() {
                for child in self.children(id) {
                    stack.push(*child);
                    let node = self.node(*child);
                    if node.family == Family::Chop
                        && pattern_matches_operator(reference, &node.name, &self.path(*child))
                    {
                        matches.push(*child);
                    }
                }
            }
            return matches;
        }
        self.resolve_ref_path(from, reference)
            .into_iter()
            .collect()
    }

    fn resolve_ref_path(&self, from: NodeId, path: &str) -> Option<NodeId> {
        let path = path.trim();
        if path.is_empty() {
            return None;
        }
        if path.starts_with('/') {
            self.find(path)
        } else {
            self.find_from(from, path)
        }
    }

    /// Resolve a path the way an operator's reference parameter reads it:
    /// `/x/y` from the root, anything else from the referencing node's own
    /// component, with `..` stepping out. A component whose Feedback targets
    /// `out1` therefore works in every instance and in every project — an
    /// absolute path would nail it to one address.
    pub fn find_from(&self, from: NodeId, path: &str) -> Option<NodeId> {
        let path = path.trim();
        if path.starts_with('/') {
            return self.find(path);
        }
        let mut cur = self.nodes.get(from)?.parent.unwrap_or(self.root);
        for seg in path.split('/').filter(|s| !s.is_empty()) {
            match seg {
                "." => {}
                ".." => cur = self.nodes[cur].parent.unwrap_or(self.root),
                _ => {
                    cur = *self.nodes[cur]
                        .children
                        .iter()
                        .find(|c| self.nodes[**c].name == seg)?;
                }
            }
        }
        Some(cur)
    }

    pub fn is_inside_external(&self, id: NodeId) -> bool {
        let mut cur = self.get(id).and_then(|n| n.parent);
        while let Some(c) = cur {
            let Some(node) = self.get(c) else {
                return false;
            };
            if node.external.is_some() {
                return true;
            }
            cur = node.parent;
        }
        false
    }
}
