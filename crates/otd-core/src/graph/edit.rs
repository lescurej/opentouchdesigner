//! Graph mutations: create, wire, parameters.

use std::collections::HashSet;

use crate::param::Param;
use crate::value::Value;

use super::{Graph, GraphError, Node, NodeFlags, NodeId, OpDef};

impl Graph {

    // ------------------------------------------------------------- creation

    pub fn create(
        &mut self,
        parent: NodeId,
        def: &OpDef,
        name: Option<&str>,
    ) -> Result<NodeId, GraphError> {
        if !self.nodes.contains_key(parent) {
            return Err(GraphError::NoSuchNode);
        }
        // An explicitly requested name is honoured exactly when it is free —
        // loading a project must not rename `myBlur` to `myBlur1`. Only a
        // collision, or a create with no name at all, gets auto-numbered.
        let name = match name {
            Some(n) if !self.name_taken(parent, n) => n.to_string(),
            Some(n) => self.unique_name(parent, n),
            // `noiseTOP` creates `noise1`, not `noiseTOP1` — the family is
            // already obvious from the node's colour and its wires.
            None => {
                let base = def
                    .type_name
                    .strip_suffix(def.family.suffix())
                    .filter(|s| !s.is_empty())
                    .unwrap_or(def.type_name);
                self.unique_name(parent, base)
            }
        };
        let node = Node {
            name,
            op_type: def.type_name.to_string(),
            family: def.family,
            params: (def.params)(),
            inputs: vec![None; def.inputs.len()],
            input_source_out: vec![0; def.inputs.len()],
            input_labels: def.inputs.iter().map(|s| s.to_string()).collect(),
            input_families: def.accepted_families(),
            connector: def.connector,
            parent: Some(parent),
            children: Vec::new(),
            pos: [0.0, 0.0],
            size: None,
            flags: NodeFlags::default(),
            intrinsically_time_dependent: def.time_dependent,
            external: None,
            clone_of: None,
            clone_synced_at: 0,
            revision: 1,
            callback_owner: None,
            docked_to: None,
            dock_shown: true,
        };
        let id = self.nodes.insert(node);
        self.nodes[parent].children.push(id);
        // A new In or Out operator changes its component's shape.
        self.sync_connectors(parent);
        Ok(id)
    }

    pub fn sync_multi_inputs(&mut self, id: NodeId) {
        self.grow_multi_inputs(id, 0);
    }

    fn grow_multi_inputs(&mut self, id: NodeId, min_slots: usize) {
        let Some(node) = self.nodes.get(id) else {
            return;
        };
        let op = node.op_type.as_str();
        if !matches!(
            op,
            "mergeCHOP"
                | "mathCHOP"
                | "switchCHOP"
                | "selectCHOP"
                | "mergeDAT"
                | "selectDAT"
        ) {
            return;
        }
        let max_used = node
            .inputs
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.is_some())
            .map(|(i, _)| i)
            .max()
            .unwrap_or(0);
        let want = (max_used + 2).max(2).max(min_slots);
        let node = &mut self.nodes[id];
        if node.inputs.len() >= want {
            return;
        }
        let numeric = node.input_labels.first().is_some_and(|s| s == "0");
        while node.input_labels.len() < want {
            let i = node.input_labels.len();
            let label = if numeric {
                i.to_string()
            } else if i < 26 {
                ((b'a' + i as u8) as char).to_string()
            } else {
                format!("in{i}")
            };
            node.input_labels.push(label);
        }
        node.inputs.resize(want, None);
        node.input_source_out.resize(want, 0);
        node.input_families.resize(want, node.family);
        node.revision += 1;
    }

    /// Delete a node and everything under it, unwiring any downstream inputs.
    pub fn remove(&mut self, id: NodeId) -> Result<(), GraphError> {
        if id == self.root {
            return Err(GraphError::NoSuchNode);
        }
        if !self.nodes.contains_key(id) {
            return Err(GraphError::NoSuchNode);
        }
        let mut doomed = vec![id];
        let mut i = 0;
        while i < doomed.len() {
            let cur = doomed[i];
            doomed.extend_from_slice(&self.nodes[cur].children.clone());
            i += 1;
        }
        doomed.extend(self.owned_callback_dats(&doomed));
        let doomed_set: HashSet<NodeId> = doomed.iter().copied().collect();
        let parent = self.nodes[id].parent;

        for d in &doomed {
            if let Some(parent) = self.nodes.get(*d).and_then(|n| n.parent) {
                self.nodes[parent].children.retain(|c| *c != *d);
            }
        }
        for d in &doomed {
            self.nodes.remove(*d);
        }
        // Unwire anything that pointed at a deleted node.
        for (_, node) in self.nodes.iter_mut() {
            let mut touched = false;
            for slot in node.inputs.iter_mut() {
                if slot.map(|s| doomed_set.contains(&s)).unwrap_or(false) {
                    *slot = None;
                    touched = true;
                }
            }
            if touched {
                node.revision += 1;
            }
        }
        // Deleting an In operator removes a connector from its component.
        if let Some(parent) = parent {
            self.sync_connectors(parent);
        }
        Ok(())
    }


    // ------------------------------------------------------------- wiring

    pub fn connect(
        &mut self,
        src: NodeId,
        dst: NodeId,
        input_index: usize,
    ) -> Result<(), GraphError> {
        self.connect_from_output(src, 0, dst, input_index)
    }

    pub fn connect_from_output(
        &mut self,
        src: NodeId,
        src_out: usize,
        dst: NodeId,
        input_index: usize,
    ) -> Result<(), GraphError> {
        if !self.nodes.contains_key(src) || !self.nodes.contains_key(dst) {
            return Err(GraphError::NoSuchNode);
        }
        if input_index >= self.nodes[dst].inputs.len() {
            self.grow_multi_inputs(dst, input_index + 1);
        }
        if input_index >= self.nodes[dst].inputs.len() {
            return Err(GraphError::BadInputIndex(input_index));
        }
        let sf = self
            .output_family_at(src, src_out)
            .ok_or(GraphError::FamilyMismatch("nothing", "an input"))?;
        let df = self.nodes[dst].input_families[input_index];
        if sf != df {
            return Err(GraphError::FamilyMismatch(sf.suffix(), df.suffix()));
        }
        if src == dst || self.reaches_upstream(src, dst) {
            return Err(GraphError::WouldCycle);
        }
        let n = &mut self.nodes[dst];
        n.inputs[input_index] = Some(src);
        if n.input_source_out.len() <= input_index {
            n.input_source_out.resize(input_index + 1, 0);
        }
        n.input_source_out[input_index] = src_out;
        n.revision += 1;
        self.audio_topology_gen = self.audio_topology_gen.wrapping_add(1);
        self.sync_multi_inputs(dst);
        Ok(())
    }

    pub fn disconnect(&mut self, dst: NodeId, input_index: usize) -> Result<(), GraphError> {
        let n = self.nodes.get_mut(dst).ok_or(GraphError::NoSuchNode)?;
        if input_index >= n.inputs.len() {
            return Err(GraphError::BadInputIndex(input_index));
        }
        n.inputs[input_index] = None;
        n.revision += 1;
        self.audio_topology_gen = self.audio_topology_gen.wrapping_add(1);
        Ok(())
    }

    /// Is `target` anywhere upstream of `from` (following inputs)?
    fn reaches_upstream(&self, from: NodeId, target: NodeId) -> bool {
        let mut stack = vec![from];
        let mut seen = HashSet::new();
        while let Some(cur) = stack.pop() {
            if cur == target {
                return true;
            }
            if !seen.insert(cur) {
                continue;
            }
            for input in self.nodes[cur].inputs.iter().flatten() {
                stack.push(*input);
            }
        }
        false
    }

    // ---------------------------------------------------------- parameters

    pub fn set_param(&mut self, id: NodeId, key: &str, value: Value) -> Result<(), GraphError> {
        let n = self.nodes.get_mut(id).ok_or(GraphError::NoSuchNode)?;
        let custom = n.params.get(key).map(|p| p.custom).unwrap_or(false);
        if let Some(p) = n.params.get_mut(key) {
            p.set_constant(value);
            n.revision += 1;
        }
        if custom {
            self.dirty_descendants(id);
        }
        Ok(())
    }

    /// Add an author-defined parameter. On a component this is how it gets an
    /// API: operators inside read it as `parent.<key>`.
    pub fn add_custom_param(&mut self, id: NodeId, key: &str, param: Param) {
        if let Some(n) = self.nodes.get_mut(id) {
            n.params.insert(key.to_string(), param.as_custom());
            n.revision += 1;
        }
        self.dirty_descendants(id);
    }

    /// Remove an author-defined parameter. Operator-defined ones are left
    /// alone — deleting those would leave the operator unable to cook.
    pub fn remove_custom_param(&mut self, id: NodeId, key: &str) {
        let is_custom = self
            .nodes
            .get(id)
            .and_then(|n| n.params.get(key))
            .map(|p| p.custom)
            .unwrap_or(false);
        if !is_custom {
            return;
        }
        if let Some(n) = self.nodes.get_mut(id) {
            n.params.shift_remove(key);
            n.revision += 1;
        }
        self.dirty_descendants(id);
    }

    /// Mark everything under `id` as changed.
    ///
    /// A component's custom parameters are read by expressions inside it, and
    /// those reads are not wires, so there is nothing for the cook engine to
    /// follow. Touching the subtree on edit is O(subtree) once per edit rather
    /// than per frame, which is the right trade.
    fn dirty_descendants(&mut self, id: NodeId) {
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let children = match self.nodes.get(cur) {
                Some(n) => n.children.clone(),
                None => continue,
            };
            for c in children {
                self.nodes[c].revision += 1;
                stack.push(c);
            }
        }
    }

    pub fn set_expression(&mut self, id: NodeId, key: &str, src: &str) -> Result<(), GraphError> {
        let n = self.nodes.get_mut(id).ok_or(GraphError::NoSuchNode)?;
        let custom = n.params.get(key).map(|p| p.custom).unwrap_or(false);
        if let Some(p) = n.params.get_mut(key) {
            p.set_expression(src);
            n.revision += 1;
        }
        if custom {
            self.dirty_descendants(id);
        }
        Ok(())
    }

    /// Put a parameter into Export mode, reading `channel` off the CHOP at
    /// `op_path`.
    ///
    /// The same edit the parameter panel makes when a channel is dragged onto
    /// a row, reached without a pointer — which is what the assistant needs,
    /// since Export is how anything in this program reacts to anything else
    /// and a plan that cannot express it can only build patches that sit
    /// still.
    pub fn set_export(
        &mut self,
        id: NodeId,
        key: &str,
        op_path: &str,
        channel: &str,
    ) -> Result<(), GraphError> {
        let n = self.nodes.get_mut(id).ok_or(GraphError::NoSuchNode)?;
        let custom = n.params.get(key).map(|p| p.custom).unwrap_or(false);
        let Some(p) = n.params.get_mut(key) else {
            return Err(GraphError::NoSuchParam(key.to_string()));
        };
        p.set_export(op_path, channel);
        n.revision += 1;
        if custom {
            self.dirty_descendants(id);
        }
        Ok(())
    }
}
