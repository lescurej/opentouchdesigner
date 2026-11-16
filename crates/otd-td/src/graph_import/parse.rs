use std::collections::BTreeMap;

use otd_core::graph::{Family, Graph, NodeId};
use otd_core::value::Value;

pub(crate) fn detect_prefix(files: &BTreeMap<String, Vec<u8>>) -> Option<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for key in files.keys() {
        if let Some((prefix, rest)) = key.split_once('/') {
            if rest.ends_with(".n") || files.contains_key(&format!("{prefix}.n")) {
                *counts.entry(prefix.to_string()).or_default() += 1;
            }
        }
    }
    counts.into_iter().max_by_key(|(_, n)| *n).map(|(p, _)| p)
}

pub(crate) fn split_td_path(td_path: &str) -> Option<(&str, &str)> {
    let slash = td_path.rfind('/')?;
    Some((&td_path[..slash], &td_path[slash + 1..]))
}

pub(crate) fn resolve_input_ref(
    graph: &Graph,
    _prefix: &str,
    parent_td: &str,
    reference: &str,
    by_td: &BTreeMap<String, NodeId>,
) -> Option<NodeId> {
    if let Some((comp, _op)) = reference.split_once('/') {
        let comp_path = format!("{parent_td}/{comp}");
        if let Some(&comp_id) = by_td.get(&comp_path) {
            if graph.node(comp_id).family == Family::Comp {
                return Some(comp_id);
            }
        }
        let path = format!("{parent_td}/{reference}");
        if let Some(&id) = by_td.get(&path) {
            return Some(id);
        }
    }
    if let Some(sibling) = reference.strip_prefix("../") {
        let parent = parent_td
            .rsplit_once('/')
            .map(|(p, _)| p)
            .unwrap_or(parent_td);
        let path = if parent.is_empty() {
            sibling.to_string()
        } else {
            format!("{parent}/{sibling}")
        };
        return by_td.get(&path).copied();
    }
    let path = format!("{parent_td}/{reference}");
    by_td.get(&path).copied()
}

pub(crate) fn dat_text_sidecar(files: &BTreeMap<String, Vec<u8>>, td_path: &str) -> Option<String> {
    let key = format!("{td_path}.text");
    let bytes = files.get(&key)?;
    Some(crate::dat_text::decode_td_dat_text(bytes))
}

pub(crate) fn data_sidecar_rel(
    files: &BTreeMap<String, Vec<u8>>,
    td_path: &str,
    name: &str,
) -> Option<String> {
    let direct = format!("{td_path}.data");
    if files.contains_key(&direct) {
        return Some(direct);
    }
    let nested = format!("{td_path}/{name}.data");
    if files.contains_key(&nested) {
        return Some(nested);
    }
    None
}

pub(crate) fn apply_n_file(graph: &mut Graph, id: NodeId, bytes: &[u8]) {
    let text = String::from_utf8_lossy(bytes);
    if let Some(pos) = parse_tile(&text) {
        graph.node_mut(id).pos = pos;
    }
    for flag in parse_flags(&text) {
        match flag {
            NodeFlag::Viewer => graph.node_mut(id).flags.viewer = true,
            NodeFlag::Render => graph.node_mut(id).flags.render = true,
            NodeFlag::Display => graph.node_mut(id).flags.viewer = true,
            NodeFlag::Lock => graph.node_mut(id).flags.lock = true,
        }
    }
}

pub(crate) enum NodeFlag {
    Viewer,
    Render,
    Display,
    Lock,
}

pub(crate) fn parse_flags(text: &str) -> Vec<NodeFlag> {
    let mut out = Vec::new();
    let Some(line) = text.lines().find(|l| l.trim_start().starts_with("flags")) else {
        return out;
    };
    if line.contains("lock on") {
        out.push(NodeFlag::Lock);
    }
    if line.contains("viewer") {
        out.push(NodeFlag::Viewer);
    }
    if line.contains("render on") {
        out.push(NodeFlag::Render);
    }
    if line.contains("display on") {
        out.push(NodeFlag::Display);
    }
    out
}

pub(crate) fn parse_dock(text: &str) -> Option<String> {
    let line = text.lines().find(|l| l.trim_start().starts_with("dock "))?;
    let name = line.split_whitespace().nth(1)?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub(crate) fn parse_dock_shown(text: &str) -> bool {
    let Some(line) = text.lines().find(|l| l.trim_start().starts_with("flags")) else {
        return true;
    };
    let parts: Vec<&str> = line.split_whitespace().collect();
    !parts.windows(2).any(|w| w[0] == "showDocked" && w[1] == "off")
}

pub(crate) fn attach_docks(graph: &mut Graph, docks: &[(NodeId, String, bool)]) {
    for (id, name, shown) in docks {
        let Some(parent) = graph.node(*id).parent else {
            continue;
        };
        let owner = graph
            .children(parent)
            .iter()
            .copied()
            .find(|child| graph.node(*child).name == *name && *child != *id);
        let Some(owner) = owner else {
            continue;
        };
        let node = graph.node_mut(*id);
        node.docked_to = Some(owner);
        node.dock_shown = *shown;
    }
}

pub(crate) fn place_docked(graph: &mut Graph) {
    let mut groups: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
    for (id, node) in graph.iter() {
        if let Some(owner) = node.docked_to {
            groups.entry(owner).or_default().push(id);
        }
    }
    for (owner, mut docks) in groups {
        if !graph.contains(owner) {
            continue;
        }
        docks.sort_by_key(|id| {
            let name = graph.node(*id).name.as_str();
            let rank = if name.ends_with("_pixel") {
                0
            } else if name.ends_with("_info") {
                1
            } else if name.ends_with("_compute") {
                2
            } else {
                3
            };
            (rank, name.to_string())
        });
        let origin = graph.node(owner).pos;
        for (i, id) in docks.iter().enumerate() {
            graph.node_mut(*id).pos = [origin[0] + 200.0, origin[1] + i as f32 * 120.0];
        }
    }
}

pub(crate) fn copy_glsl_pixel_sources(graph: &mut Graph) {
    let glsl: Vec<NodeId> = graph
        .iter()
        .filter(|(_, node)| node.op_type == "glslTOP")
        .map(|(id, _)| id)
        .collect();
    for id in glsl {
        let _ = graph.set_param(id, "language", Value::Select("glsl".into()));
        let pixel = graph
            .node(id)
            .params
            .get("pixeldat")
            .map(|p| p.value.as_str())
            .unwrap_or_default();
        let pixel = pixel.trim().to_string();
        if pixel.is_empty() {
            continue;
        }
        let Some(parent) = graph.node(id).parent else {
            continue;
        };
        let dat = graph
            .children(parent)
            .iter()
            .copied()
            .find(|child| graph.node(*child).name == pixel);
        let Some(dat) = dat else {
            continue;
        };
        let text = graph
            .node(dat)
            .params
            .get("text")
            .map(|p| p.value.as_str())
            .unwrap_or_default();
        if text.trim().is_empty() {
            continue;
        }
        let _ = graph.set_param(id, "source", Value::Str(text));
    }
}

pub(crate) fn parse_tile(text: &str) -> Option<[f32; 2]> {
    let line = text.lines().find(|l| l.trim_start().starts_with("tile "))?;
    let mut parts = line.split_whitespace();
    parts.next()?;
    let x: f32 = parts.next()?.parse().ok()?;
    let y: f32 = parts.next()?.parse().ok()?;
    Some([x, y])
}

pub(crate) fn parse_wire_block(text: &str, marker: &str) -> Vec<(usize, String)> {
    let Some(start) = text.find(marker) else {
        return Vec::new();
    };
    let rest = &text[start..];
    let Some(open) = rest.find('{').or_else(|| rest.find('\n')) else {
        return Vec::new();
    };
    let body = &rest[open..];
    let close = body.find('}').unwrap_or(body.len());
    let body = &body[..close];
    let mut inputs = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line == "{" || line == "}" {
            continue;
        }
        let mut parts = line.split_whitespace();
        let index: usize = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let name = parts.next().unwrap_or("").to_string();
        if !name.is_empty() {
            inputs.push((index, name));
        }
    }
    inputs
}

pub(crate) fn parse_inputs(text: &str) -> Vec<(usize, String)> {
    if let Some(i) = text.find("inputs") {
        // Avoid treating `compinputs` as a normal `inputs` block (`.network` files).
        if !text[i..].starts_with("compinputs") {
            return parse_wire_block(text, "inputs");
        }
    }
    parse_wire_block(text, "input")
}

pub(crate) fn parse_compinputs(text: &str) -> Vec<(usize, String)> {
    parse_wire_block(text, "compinputs")
}
