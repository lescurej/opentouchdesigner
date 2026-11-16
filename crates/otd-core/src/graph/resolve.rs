//! Output and connector resolution.

use std::collections::HashSet;

use super::{Connector, Family, Graph, NodeId};

impl Graph {
    // -------------------------------------------------- component connectors

    /// The In (or Out) operators of a component, in name order, which is the
    /// order their connectors appear on the node.
    pub fn connector_ops(&self, comp: NodeId, kind: Connector) -> Vec<NodeId> {
        let Some(node) = self.nodes.get(comp) else {
            return Vec::new();
        };
        let mut ops: Vec<NodeId> = node
            .children
            .iter()
            .copied()
            .filter(|c| self.nodes[*c].connector == kind)
            .collect();
        ops.sort_by(|a, b| self.nodes[*a].name.cmp(&self.nodes[*b].name));
        ops
    }

    /// Recompute a component's input connectors from the In operators inside
    /// it. Existing connections keep their index, so adding a second input
    /// does not unwire the first.
    pub fn sync_connectors(&mut self, comp: NodeId) {
        if self.nodes.get(comp).map(|n| n.family) != Some(Family::Comp) {
            return;
        }
        let ins = self.connector_ops(comp, Connector::In);
        let labels: Vec<String> = ins.iter().map(|i| self.nodes[*i].name.clone()).collect();
        let families: Vec<Family> = ins.iter().map(|i| self.nodes[*i].family).collect();

        let node = &mut self.nodes[comp];
        if node.input_labels == labels && node.input_families == families {
            return;
        }
        node.inputs.resize(ins.len(), None);
        node.input_source_out.resize(ins.len(), 0);
        node.input_labels = labels;
        node.input_families = families;
        node.revision += 1;
    }

    /// Out operators inside a component, in connector order (name-sorted).
    pub fn comp_out_connectors(&self, comp: NodeId) -> Vec<(NodeId, Family, String)> {
        self.connector_ops(comp, Connector::Out)
            .into_iter()
            .map(|id| {
                let n = &self.nodes[id];
                (id, n.family, n.name.clone())
            })
            .collect()
    }

    /// The family a node presents on its output: its own, or — for a
    /// component — that of the Out operator inside it.
    pub fn output_family(&self, id: NodeId) -> Option<Family> {
        self.output_family_at(id, 0)
    }

    pub fn output_family_at(&self, id: NodeId, out_index: usize) -> Option<Family> {
        let node = self.nodes.get(id)?;
        if node.family != Family::Comp {
            return Some(node.family);
        }
        self.connector_ops(id, Connector::Out)
            .get(out_index)
            .map(|o| self.nodes[*o].family)
    }

    /// The node that actually produces what `id` presents, following bypass
    /// flags and stepping out of components into their Out operator.
    pub fn resolve_output(&self, id: NodeId) -> Option<NodeId> {
        self.resolve_output_at(id, 0)
    }

    pub fn resolve_output_at(&self, id: NodeId, out_index: usize) -> Option<NodeId> {
        let mut cur = id;
        let mut out = out_index;
        for _ in 0..64 {
            let node = self.nodes.get(cur)?;
            if node.flags.bypass {
                cur = (*node.inputs.first()?)?;
                out = *node.input_source_out.first()?;
                continue;
            }
            if node.family == Family::Comp {
                cur = *self.connector_ops(cur, Connector::Out).get(out)?;
                out = 0;
                continue;
            }
            return Some(cur);
        }
        None
    }

    pub fn resolve_input_source(&self, dst: NodeId, input_index: usize) -> Option<NodeId> {
        let src = (*self.nodes.get(dst)?.inputs.get(input_index)?)?;
        let out = self
            .nodes
            .get(dst)?
            .input_source_out
            .get(input_index)
            .copied()
            .unwrap_or(0);
        self.resolve_output_at(src, out)
    }

    /// For an In operator, the node outside the component that feeds it.
    pub fn connector_source(&self, in_op: NodeId) -> Option<NodeId> {
        let node = self.nodes.get(in_op)?;
        if node.connector != Connector::In {
            return None;
        }
        let comp = node.parent?;
        let index = self
            .connector_ops(comp, Connector::In)
            .iter()
            .position(|i| *i == in_op)?;
        let outside = (*self.nodes.get(comp)?.inputs.get(index)?)?;
        self.resolve_output(outside)
    }

    pub fn name_taken(&self, parent: NodeId, name: &str) -> bool {
        self.nodes[parent]
            .children
            .iter()
            .any(|c| self.nodes[*c].name == name)
    }

    /// Auto-numbered names, TD style: `noise1`, `noise2`, ...
    pub fn unique_name(&self, parent: NodeId, base: &str) -> String {
        let base = base.trim_end_matches(|c: char| c.is_ascii_digit());
        let taken: HashSet<&str> = self.nodes[parent]
            .children
            .iter()
            .map(|c| self.nodes[*c].name.as_str())
            .collect();
        for i in 1.. {
            let candidate = format!("{base}{i}");
            if !taken.contains(candidate.as_str()) {
                return candidate;
            }
        }
        unreachable!()
    }

    pub fn consumers(&self, id: NodeId) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.inputs.iter().flatten().any(|i| *i == id))
            .map(|(k, _)| k)
            .collect()
    }
}
