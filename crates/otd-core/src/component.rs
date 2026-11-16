//! External component files (`.otdc`) and clones.
//!
//! PLAN.md Phase 3 asks for "external component files (.otdc, text) +
//! git-friendly merges as a showcase". Two mechanisms, with one thing in
//! common: a component's *contents* stop being duplicated.
//!
//!  * An **external** component keeps its network in its own file. The
//!    project stores a path and the settings of the instance; the file stores
//!    the network. Two projects using the same component share one file, and
//!    a change to it is a diff in one place.
//!  * A **clone** tracks another component in the same project. When the
//!    master changes, only the network *inside* the clone is re-copied; the
//!    clone COMP's own custom parameters stay on the instance and are not
//!    re-synced from the master after the first expand.
//!
//! Both preserve the instance's own settings across a re-expansion. Losing
//! the values an artist dialled in because a shared definition changed would
//! make either feature unusable.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::graph::{Graph, NodeId, OpRegistry};
use crate::param::Param;
use crate::project::{NodeEntry, ProjectError, node_entry, restore_node};

pub const COMPONENT_FORMAT_ID: &str = "opentouchdesigner-component";
pub const COMPONENT_FORMAT_VERSION: u32 = 1;

/// A component saved on its own: the operators inside it, plus the custom
/// parameters that make up its API.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Component {
    pub format: String,
    pub version: u32,
    /// What the component was called where it was saved from. Used as the
    /// default name when instantiating.
    pub name: String,
    /// The component's own API. Values here are defaults for a new instance.
    #[serde(default, skip_serializing_if = "indexmap::IndexMap::is_empty")]
    pub params: indexmap::IndexMap<String, Param>,
    /// The network inside, with paths relative to the component.
    pub nodes: Vec<NodeEntry>,
}

impl Component {
    /// Extract a component from a graph.
    pub fn from_graph(graph: &Graph, comp: NodeId, registry: &OpRegistry) -> Option<Component> {
        let node = graph.get(comp)?;
        let prefix = graph.path(comp);

        // Path-sorted, like the project format, so a parent is always written
        // before its children and the file is stable.
        let mut entries: BTreeMap<String, NodeEntry> = BTreeMap::new();
        let mut stack: Vec<NodeId> = node.children.clone();
        while let Some(id) = stack.pop() {
            stack.extend(graph.node(id).children.iter().copied());
            let mut entry = node_entry(graph, id, registry);
            entry.path = strip_prefix(&entry.path, &prefix);
            entry.inputs = entry
                .inputs
                .iter()
                .map(|i| {
                    if i.is_empty() {
                        String::new()
                    } else {
                        strip_prefix(i, &prefix)
                    }
                })
                .collect();
            entries.insert(entry.path.clone(), entry);
        }

        Some(Component {
            format: COMPONENT_FORMAT_ID.to_string(),
            version: COMPONENT_FORMAT_VERSION,
            name: node.name.clone(),
            params: node
                .custom_params()
                .map(|(k, p)| (k.clone(), p.clone()))
                .collect(),
            nodes: entries.into_values().collect(),
        })
    }

    /// Fill a component node with this component's contents.
    ///
    /// Existing children are removed first, and the node's own custom
    /// parameter *values* are preserved where the names still match — an
    /// instance keeps its settings when the definition it came from is
    /// re-read.
    pub fn expand_into(
        &self,
        graph: &mut Graph,
        comp: NodeId,
        registry: &OpRegistry,
    ) -> Result<(), ProjectError> {
        self.expand_into_with(graph, comp, registry, true)
    }

    /// Like [`expand_into`], but when `sync_custom_params` is false only the
    /// child network is replaced — used when a clone follows its master.
    pub fn expand_into_with(
        &self,
        graph: &mut Graph,
        comp: NodeId,
        registry: &OpRegistry,
        sync_custom_params: bool,
    ) -> Result<(), ProjectError> {
        if self.format != COMPONENT_FORMAT_ID {
            return Err(ProjectError::NotAProject);
        }
        if self.version > COMPONENT_FORMAT_VERSION {
            return Err(ProjectError::TooNew(self.version));
        }

        let kept: Vec<(String, Param)> = if sync_custom_params {
            graph
                .get(comp)
                .map(|n| {
                    n.custom_params()
                        .map(|(k, p)| (k.clone(), p.clone()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Wires from the parent network into this component are not part of the
        // saved snapshot. Removing In operators runs sync_connectors and can
        // clear them while children are torn down, so remember them first.
        let external: Vec<(String, Option<NodeId>, usize)> = graph
            .get(comp)
            .map(|n| {
                n.input_labels
                    .iter()
                    .enumerate()
                    .map(|(i, label)| {
                        let src = n.inputs.get(i).copied().flatten();
                        let out = n.input_source_out.get(i).copied().unwrap_or(0);
                        (label.clone(), src, out)
                    })
                    .collect()
            })
            .unwrap_or_default();

        for child in graph.children(comp).to_vec() {
            let _ = graph.remove(child);
        }

        if sync_custom_params {
            // The API first, so expressions inside can resolve `parent.x` as soon
            // as they are created.
            for (key, param) in &self.params {
                let mut param = param.clone();
                if let Some((_, existing)) = kept.iter().find(|(k, _)| k == key) {
                    if existing.value.same_type_as(&param.value) {
                        param.value = existing.value.clone();
                        param.mode = existing.mode;
                        param.expression = existing.expression.clone();
                        param.recompile();
                    }
                }
                graph.add_custom_param(comp, key, param);
            }
        }

        let mut by_path: BTreeMap<String, NodeId> = BTreeMap::new();
        by_path.insert(String::new(), comp);

        for entry in &self.nodes {
            let (parent_path, name) = split_relative(&entry.path)
                .ok_or_else(|| ProjectError::BadPath(entry.path.clone()))?;
            let parent = *by_path
                .get(&parent_path)
                .ok_or_else(|| ProjectError::BadPath(entry.path.clone()))?;
            let resolved =
                crate::missing::resolve_op_def(registry, &entry.op).ok_or_else(|| {
                    ProjectError::UnknownOp {
                        op: entry.op.clone(),
                        path: entry.path.clone(),
                    }
                })?;
            let id = graph
                .create(parent, resolved.def, Some(&name))
                .map_err(|_| ProjectError::BadPath(entry.path.clone()))?;
            restore_node(graph, id, entry);
            crate::missing::finish_missing_substitution(graph, id, &resolved);
            by_path.insert(entry.path.clone(), id);
        }

        for entry in &self.nodes {
            let dst = by_path[&entry.path];
            for (i, src_path) in entry.inputs.iter().enumerate() {
                if src_path.is_empty() {
                    continue;
                }
                let src = *by_path
                    .get(src_path)
                    .ok_or_else(|| ProjectError::DanglingInput {
                        path: entry.path.clone(),
                        input: src_path.clone(),
                    })?;
                graph
                    .connect(src, dst, i)
                    .map_err(|_| ProjectError::DanglingInput {
                        path: entry.path.clone(),
                        input: src_path.clone(),
                    })?;
            }
        }

        graph.sync_connectors(comp);
        for (label, src, src_out) in external {
            let Some(idx) = graph
                .get(comp)
                .and_then(|n| n.input_labels.iter().position(|l| l == &label))
            else {
                continue;
            };
            if let Some(src) = src {
                let _ = graph.connect_from_output(src, src_out, comp, idx);
            }
        }
        Ok(())
    }

    pub fn to_ron(&self) -> Result<String, ProjectError> {
        let cfg = ron::ser::PrettyConfig::new()
            .depth_limit(6)
            .indentor("  ")
            .struct_names(false)
            .separate_tuple_members(false)
            .enumerate_arrays(false);
        let mut s = ron::ser::to_string_pretty(self, cfg)
            .map_err(|e| ProjectError::Serialise(e.to_string()))?;
        s.push('\n');
        Ok(s)
    }

    pub fn from_ron(src: &str) -> Result<Component, ProjectError> {
        ron::from_str(src).map_err(|e| ProjectError::Parse(e.to_string()))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ProjectError> {
        std::fs::write(path, self.to_ron()?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Component, ProjectError> {
        Component::from_ron(&std::fs::read_to_string(path)?)
    }
}

fn strip_prefix(path: &str, prefix: &str) -> String {
    path.strip_prefix(prefix).unwrap_or(path).to_string()
}

/// `/level1` -> (``, `level1`); `/inner/level1` -> (`/inner`, `level1`).
fn split_relative(path: &str) -> Option<(String, String)> {
    let trimmed = path.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    let name = &trimmed[idx + 1..];
    if name.is_empty() {
        return None;
    }
    Some((trimmed[..idx].to_string(), name.to_string()))
}

impl Graph {
    /// Point a component at a `.otdc` file and fill it from there.
    pub fn attach_external(
        &mut self,
        comp: NodeId,
        file: &str,
        registry: &OpRegistry,
    ) -> Result<(), ProjectError> {
        // The reference is resolved for *reading* but stored exactly as it was
        // given. A bundle holding `components/meter.otdc` has to keep saying
        // that, or the first re-save turns it into an absolute path on one
        // machine and the bundle stops being portable.
        let component = Component::load(self.resolve_external(file))?;
        component.expand_into(self, comp, registry)?;
        if let Some(n) = self.get_mut_raw(comp) {
            n.external = Some(file.to_string());
        }
        Ok(())
    }

    /// Make `comp` track `master`.
    pub fn set_clone(&mut self, comp: NodeId, master_path: Option<&str>) {
        let new = master_path.map(|s| s.to_string());
        if let Some(n) = self.get_mut_raw(comp) {
            if n.clone_of == new {
                return;
            }
            n.clone_of = new;
            n.clone_synced_at = 0;
        }
    }

    /// Sum of the revisions in a subtree — a cheap "has anything in here
    /// changed" number, so an untouched master costs one walk and no copying.
    pub fn subtree_revision(&self, id: NodeId) -> u64 {
        let mut total = 0u64;
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            let Some(node) = self.get(cur) else { continue };
            total = total.wrapping_add(node.revision());
            stack.extend(node.children.iter().copied());
        }
        total
    }

    /// Re-expand every clone whose master has changed.
    ///
    /// Returns how many were re-synced, which the editor reports so the
    /// behaviour is visible rather than mysterious.
    pub fn sync_clones(&mut self, registry: &OpRegistry) -> usize {
        let clones: Vec<(NodeId, String)> = self
            .walk()
            .into_iter()
            .filter_map(|id| {
                let node = self.get(id)?;
                Some((id, node.clone_of.clone()?))
            })
            .collect();

        let mut synced = 0;
        for (id, master_path) in clones {
            if self.get(id).is_some_and(|n| n.flags.clone_immune) {
                continue;
            }
            let Some(master) = self.find(&master_path) else {
                continue;
            };
            if master == id || self.is_ancestor(master, id) {
                // Cloning something that contains you would expand forever.
                continue;
            }
            let revision = self.subtree_revision(master);
            if self.get(id).map(|n| n.clone_synced_at) == Some(revision) {
                continue;
            }
            let Some(component) = Component::from_graph(self, master, registry) else {
                continue;
            };
            // First expand copies the master's API once; later syncs touch only
            // the network inside the clone.
            let sync_params = self.get(id).is_some_and(|n| n.clone_synced_at == 0);
            if component
                .expand_into_with(self, id, registry, sync_params)
                .is_ok()
            {
                if let Some(n) = self.get_mut_raw(id) {
                    n.clone_synced_at = revision;
                }
                synced += 1;
            }
        }
        synced
    }

    fn is_ancestor(&self, ancestor: NodeId, of: NodeId) -> bool {
        let mut cur = self.get(of).and_then(|n| n.parent);
        while let Some(c) = cur {
            if c == ancestor {
                return true;
            }
            cur = self.get(c).and_then(|n| n.parent);
        }
        false
    }
}

#[cfg(test)]
#[path = "tests/component__tests.rs"]
mod tests;
