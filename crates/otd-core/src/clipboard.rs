use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::graph::{Family, Graph, NodeId, OpRegistry};
use crate::project::{NodeEntry, ProjectError, node_entry, restore_node};

const CLIPBOARD_MAGIC: &str = "otd-op-clipboard-v1:";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeClipboard {
    pub nodes: Vec<NodeEntry>,
}

pub fn encode_operator_clipboard(clip: &NodeClipboard) -> Option<String> {
    if clip.nodes.is_empty() {
        return None;
    }
    ron::to_string(clip)
        .ok()
        .map(|body| format!("{CLIPBOARD_MAGIC}{body}"))
}

pub fn decode_operator_clipboard(text: &str) -> Option<NodeClipboard> {
    let body = text.strip_prefix(CLIPBOARD_MAGIC)?;
    ron::from_str(body).ok()
}

pub fn copy_selection(
    graph: &Graph,
    network: NodeId,
    selection: &[NodeId],
    registry: &OpRegistry,
) -> NodeClipboard {
    let set = expand_copy_set(graph, network, selection);
    if set.is_empty() {
        return NodeClipboard::default();
    }
    let prefix = network_prefix(graph, network);
    let mut id_to_rel = std::collections::HashMap::new();
    for id in &set {
        let full = graph.path(*id);
        id_to_rel.insert(*id, rel_path(&full, &prefix));
    }

    let mut nodes: Vec<NodeEntry> = set
        .iter()
        .map(|id| {
            let mut entry = node_entry(graph, *id, registry);
            entry.path = id_to_rel[id].clone();
            entry.inputs = entry
                .inputs
                .iter()
                .map(|inp| {
                    if inp.is_empty() {
                        return String::new();
                    }
                    let Some(src) = graph.find(inp) else {
                        return String::new();
                    };
                    if set.contains(&src) {
                        id_to_rel[&src].clone()
                    } else {
                        String::new()
                    }
                })
                .collect();
            entry
        })
        .collect();
    nodes.sort_by_key(|e| e.path.matches('/').count());
    NodeClipboard { nodes }
}

pub fn paste(
    graph: &mut Graph,
    parent: NodeId,
    registry: &OpRegistry,
    clip: &NodeClipboard,
    origin: [f32; 2],
) -> Result<Vec<NodeId>, ProjectError> {
    if clip.nodes.is_empty() {
        return Ok(Vec::new());
    }
    let (min_x, min_y) = clip.nodes.iter().fold((f32::MAX, f32::MAX), |(mx, my), n| {
        (mx.min(n.pos.0), my.min(n.pos.1))
    });

    let mut by_path: std::collections::BTreeMap<String, NodeId> = std::collections::BTreeMap::new();
    by_path.insert(String::new(), parent);

    let mut created = Vec::new();
    for entry in &clip.nodes {
        let (parent_path, name) =
            split_relative(&entry.path).ok_or_else(|| ProjectError::BadPath(entry.path.clone()))?;
        let parent_id = *by_path
            .get(&parent_path)
            .ok_or_else(|| ProjectError::BadPath(entry.path.clone()))?;
        let resolved = crate::missing::resolve_op_def(registry, &entry.op).ok_or_else(|| {
            ProjectError::UnknownOp {
                op: entry.op.clone(),
                path: entry.path.clone(),
            }
        })?;
        let id = graph
            .create(parent_id, resolved.def, Some(&name))
            .map_err(|_| ProjectError::BadPath(entry.path.clone()))?;
        let mut entry = entry.clone();
        entry.pos.0 += origin[0] - min_x;
        entry.pos.1 += origin[1] - min_y;
        restore_node(graph, id, &entry);
        crate::missing::finish_missing_substitution(graph, id, &resolved);
        by_path.insert(entry.path.clone(), id);
        created.push(id);
    }

    for entry in &clip.nodes {
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

    Ok(created)
}

fn expand_copy_set(graph: &Graph, network: NodeId, selection: &[NodeId]) -> BTreeSet<NodeId> {
    let mut set = BTreeSet::new();
    for &id in selection {
        if graph.node(id).flags.lock {
            continue;
        }
        if !in_network(graph, network, id) {
            continue;
        }
        set.insert(id);
        if graph.node(id).family == Family::Comp {
            let mut stack: Vec<NodeId> = graph.node(id).children.clone();
            while let Some(c) = stack.pop() {
                stack.extend(graph.node(c).children.iter().copied());
                set.insert(c);
            }
        }
    }
    set
}

fn in_network(graph: &Graph, network: NodeId, id: NodeId) -> bool {
    if id == network {
        return false;
    }
    let mut cur = Some(id);
    while let Some(n) = cur {
        if n == network {
            return true;
        }
        cur = graph.get(n).and_then(|node| node.parent);
    }
    false
}

fn network_prefix(graph: &Graph, network: NodeId) -> String {
    if network == graph.root() {
        String::new()
    } else {
        graph.path(network)
    }
}

fn rel_path(full: &str, prefix: &str) -> String {
    let rest = if prefix.is_empty() {
        full.to_string()
    } else {
        let p = prefix.trim_end_matches('/');
        full.strip_prefix(p)
            .or_else(|| full.strip_prefix(&format!("{p}/")))
            .unwrap_or(full)
            .to_string()
    };
    rest.trim_start_matches('/').to_string()
}

fn split_relative(path: &str) -> Option<(String, String)> {
    let trimmed = path.trim().trim_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.rfind('/') {
        None => Some((String::new(), trimmed.to_string())),
        Some(idx) => {
            let name = &trimmed[idx + 1..];
            if name.is_empty() {
                return None;
            }
            Some((trimmed[..idx].to_string(), name.to_string()))
        }
    }
}

#[cfg(test)]
#[path = "tests/clipboard__tests.rs"]
mod tests;
