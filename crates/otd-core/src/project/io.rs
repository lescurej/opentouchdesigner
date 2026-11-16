use indexmap::IndexMap;
use std::collections::BTreeMap;
use std::path::Path;

use crate::graph::{Graph, NodeId, OpRegistry};
use crate::value::Value;

use super::types::{
    FORMAT_ID, FORMAT_VERSION, NodeEntry, Project, ProjectEditor, ProjectError, ProjectSettings,
    PROJECT_SAMPLE_RATE_DEFAULT, param_matches_default,
};

impl Project {
    /// Snapshot a graph. Parameters left at their operator defaults are not
    /// written — see the module docs.
    ///
    /// The editor camera is not on the graph. A caller that rewrites an
    /// existing file copies [`Project::editor`] across after this.
    pub fn from_graph(graph: &Graph, registry: &OpRegistry, fps: f64) -> Project {
        // BTreeMap gives a deterministic, path-sorted file, and guarantees a
        // parent is written before its children on load.
        let mut entries: BTreeMap<String, NodeEntry> = BTreeMap::new();

        for id in graph.walk() {
            if id == graph.root() {
                continue;
            }
            // The contents of an external component live in its own file, so
            // the project references it rather than duplicating it.
            if graph.is_inside_external(id) {
                continue;
            }
            let entry = node_entry(graph, id, registry);
            entries.insert(entry.path.clone(), entry);
        }

        Project {
            format: FORMAT_ID.to_string(),
            version: FORMAT_VERSION,
            fps,
            sample_rate: PROJECT_SAMPLE_RATE_DEFAULT,
            nodes: entries.into_values().collect(),
            settings: ProjectSettings::default(),
            editor: ProjectEditor::default(),
        }
    }

    /// Rebuild a graph. Unknown operator types become family placeholders;
    /// see [`crate::missing`]. Fails on dangling wires, not on missing types.
    pub fn to_graph(&self, registry: &OpRegistry) -> Result<Graph, ProjectError> {
        self.to_graph_relative_to(registry, None)
    }

    /// Load a project file, resolving its component references against the
    /// directory the file is in.
    ///
    /// This is what `to_graph` alone cannot do: a project holding
    /// `components/meter.otdc` has to find that file next to *itself*, not
    /// next to whatever directory the process happens to be running in. Get
    /// that wrong and a project bundle works when you double-click it and
    /// fails when a show machine runs it from a service.
    pub fn open(path: impl AsRef<Path>, registry: &OpRegistry) -> Result<Graph, ProjectError> {
        let path = path.as_ref();
        Project::load(path)?.to_graph_relative_to(registry, path.parent())
    }

    /// Rebuild a graph, resolving relative external component paths against
    /// `base`. Absolute paths are used as they are.
    pub fn to_graph_relative_to(
        &self,
        registry: &OpRegistry,
        base: Option<&Path>,
    ) -> Result<Graph, ProjectError> {
        if self.format != FORMAT_ID {
            return Err(ProjectError::NotAProject);
        }
        if self.version > FORMAT_VERSION {
            return Err(ProjectError::TooNew(self.version));
        }

        let mut graph = Graph::new();
        graph.set_base_dir(base.map(|p| p.to_path_buf()));
        let mut by_path: BTreeMap<String, NodeId> = BTreeMap::new();

        // Pass 1 — create every node. Entries are path-sorted, so a parent
        // always exists by the time its children are read.
        for entry in &self.nodes {
            let (parent_path, name) =
                split_path(&entry.path).ok_or_else(|| ProjectError::BadPath(entry.path.clone()))?;
            let parent = if parent_path.is_empty() {
                graph.root()
            } else {
                *by_path
                    .get(&parent_path)
                    .ok_or_else(|| ProjectError::BadPath(entry.path.clone()))?
            };
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

            restore_node(&mut graph, id, entry);
            crate::missing::finish_missing_substitution(&mut graph, id, &resolved);

            // An external component's network is not in this file: read it
            // from the one it names. A missing file is an error rather than a
            // silently empty component.
            if let Some(file) = &entry.external {
                graph
                    .attach_external(id, file, registry)
                    .map_err(|e| ProjectError::External {
                        path: entry.path.clone(),
                        file: file.clone(),
                        reason: e.to_string(),
                    })?;
            }
            if let Some(owner_path) = &entry.callback_owner {
                if let Some(&owner_id) = by_path.get(owner_path) {
                    graph.node_mut(id).callback_owner = Some(owner_id);
                }
            }
            if let Some(dock_path) = &entry.docked_to {
                if let Some(&dock_id) = by_path.get(dock_path) {
                    graph.node_mut(id).docked_to = Some(dock_id);
                }
            }
            graph.node_mut(id).dock_shown = entry.dock_shown;
            by_path.insert(entry.path.clone(), id);
        }

        // Pass 2 — wire.
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

        // Clones can only be resolved once every node exists.
        graph.sync_clones(registry);

        for id in graph.walk() {
            graph.sync_multi_inputs(id);
        }

        for id in graph.walk() {
            if graph.node(id).family == crate::graph::Family::Comp {
                graph.sync_connectors(id);
            }
        }

        Ok(graph)
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

    pub fn from_ron(src: &str) -> Result<Project, ProjectError> {
        ron::from_str(src).map_err(|e| ProjectError::Parse(e.to_string()))
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), ProjectError> {
        std::fs::write(path, self.to_ron()?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Project, ProjectError> {
        Project::from_ron(&std::fs::read_to_string(path)?)
    }
}

/// Build the file entry for one node.
pub(crate) fn node_entry(graph: &Graph, id: NodeId, registry: &OpRegistry) -> NodeEntry {
    let node = graph.node(id);
    let path = graph.path(id);
    let defaults = registry.get(&node.op_type).map(|d| (d.params)());

    let mut params = IndexMap::new();
    for (key, param) in &node.params {
        let is_default = defaults
            .as_ref()
            .and_then(|d| d.get(key))
            .map(|d| param_matches_default(param, d))
            .unwrap_or(false);
        // A custom parameter has no operator definition behind it, so it is
        // written in full — it *is* the definition.
        if param.custom {
            let mut p = param.clone();
            if p.is_path_ref() {
                p.op_ref = true;
            }
            params.insert(key.clone(), p);
        } else if !is_default {
            // Labels, ranges and menus belong to the operator definition, not
            // to the project. Writing them would put UI metadata in every
            // diff and make an operator's relabelling look like a change.
            let mut param = param.clone();
            param.label = String::new();
            param.range = None;
            param.logarithmic = false;
            param.log_floor = None;
            param.menu = None;
            params.insert(key.clone(), param);
        }
    }

    let inputs: Vec<String> = node
        .inputs
        .iter()
        .map(|slot| slot.map(|s| graph.path(s)).unwrap_or_default())
        .collect();

    NodeEntry {
        path,
        op: crate::missing::node_entry_op_type(node),
        pos: (node.pos[0], node.pos[1]),
        size: node.size.map(|s| (s[0], s[1])),
        // Trailing unconnected inputs carry no information.
        inputs: trim_trailing_empty(inputs),
        params,
        flags: node.flags,
        external: node.external.clone(),
        clone: node.clone_of.clone(),
        callback_owner: node.callback_owner.map(|owner| graph.path(owner)),
        docked_to: node.docked_to.map(|owner| graph.path(owner)),
        dock_shown: node.dock_shown,
    }
}

/// Apply a file entry's settings to a node that has just been created.
pub(crate) fn restore_node(graph: &mut Graph, id: NodeId, entry: &NodeEntry) {
    let node = graph.node_mut(id);
    node.pos = [entry.pos.0, entry.pos.1];
    node.size = entry.size.map(|s| [s.0, s.1]);
    node.flags = entry.flags;
    node.clone_of = entry.clone.clone();
    for (key, saved) in &entry.params {
        // A custom parameter exists only in the file; restore it whole,
        // including its type, label and range.
        if saved.custom {
            let mut saved = saved.clone();
            if saved.op_ref {
                saved = saved.into_path_ref();
            }
            saved.recompile();
            node.params.insert(key.clone(), saved);
            continue;
        }
        if let Some(slot) = node.params.get_mut(key) {
            let mut saved = saved.clone();
            // Keep the operator's declared type authoritative; a hand-edited
            // file cannot turn a float into a string. Menu choices are stored
            // as either a string or a select — the parameter panel writes a
            // select into menus the operator declared as a string — so keep
            // the chosen text instead of falling back to the default.
            if (slot.menu.is_some() || matches!(slot.value, Value::Select(_)))
                && matches!(saved.value, Value::Str(_) | Value::Select(_))
            {
                let chosen = saved.value.as_str();
                saved.value = match &slot.value {
                    Value::Select(_) => Value::Select(chosen),
                    _ => Value::Str(chosen),
                };
            } else if !slot.value.same_type_as(&saved.value) {
                saved.value = slot.value.clone();
            }
            saved.label = slot.label.clone();
            // Whether a parameter holds script source is part of the operator
            // definition, not of the project.
            if slot.is_script() {
                saved = saved.into_script();
            }
            if slot.is_path_ref() {
                saved = saved.into_path_ref();
            }
            saved.range = slot.range.or(saved.range);
            saved.logarithmic = slot.logarithmic;
            saved.log_floor = slot.log_floor;
            saved.menu = slot.menu.clone().or(saved.menu);
            // Which hardware list fills the device menu is definition
            // metadata, same as the menu itself. It is not in the file.
            saved = saved.with_audio_device_list(slot.audio_device_list());
            if slot.is_file_ref() {
                saved = saved.as_file_ref();
            }
            if slot.is_folder_ref() {
                saved = saved.as_folder_ref();
            }
            saved.recompile();
            *slot = saved;
        }
    }
}

fn trim_trailing_empty(mut v: Vec<String>) -> Vec<String> {
    while v.last().map(|s| s.is_empty()).unwrap_or(false) {
        v.pop();
    }
    v
}

/// `/a/b/c` -> (`/a/b`, `c`)
fn split_path(path: &str) -> Option<(String, String)> {
    let trimmed = path.trim_end_matches('/');
    let idx = trimmed.rfind('/')?;
    let name = &trimmed[idx + 1..];
    if name.is_empty() {
        return None;
    }
    Some((trimmed[..idx].to_string(), name.to_string()))
}
