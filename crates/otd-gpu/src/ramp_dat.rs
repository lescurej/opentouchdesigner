//! Default Table DAT paired with a Ramp TOP (TouchDesigner-style ramp keys).

use otd_core::{Graph, NodeId, OpRegistry, Value};

pub const PARAM_DAT: &str = "dat";

const TABLE: &str = "tableDAT";

const NODE_MAIN_H: f32 = 117.0;
const FOLD_STRIP_H: f32 = 16.0;
const NETWORK_GRID_STEP: f32 = 40.0;

/// Default ramp keys: black at 0, white at 1 (matches default Color 1 / Color 2).
pub const DEFAULT_RAMP_TABLE: &str = "pos\tr\tg\tb\ta\n0\t0\t0\t0\t1\n1\t1\t1\t1\t1";

pub fn default_ramp_dat_pos(owner_pos: [f32; 2]) -> [f32; 2] {
    [
        owner_pos[0],
        owner_pos[1] + NODE_MAIN_H + FOLD_STRIP_H + NETWORK_GRID_STEP,
    ]
}

pub fn resolved_ramp_dat(graph: &Graph, owner: NodeId) -> Option<NodeId> {
    let owner_node = graph.get(owner)?;
    if owner_node.op_type != "rampTOP" {
        return None;
    }
    let path = owner_node
        .param(PARAM_DAT)
        .map(|p| p.value.as_str().to_string())
        .unwrap_or_default();
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let target = if path.starts_with('/') {
        graph.find(path)
    } else {
        graph.find_from(owner, path)
    }?;
    let node = graph.get(target)?;
    (node.op_type == TABLE).then_some(target)
}

pub fn install_ramp_dat(graph: &mut Graph, registry: &OpRegistry, owner: NodeId) -> Option<NodeId> {
    let owner_node = graph.get(owner)?;
    if owner_node.op_type != "rampTOP" {
        return None;
    }
    let already = owner_node
        .param(PARAM_DAT)
        .map(|p| !p.value.as_str().trim().is_empty())
        .unwrap_or(false);
    if already {
        return None;
    }
    let owner_name = owner_node.name.clone();
    let owner_pos = owner_node.pos;
    let parent = owner_node.parent?;

    let def = registry.get(TABLE)?.clone();
    let dat_name = format!("{owner_name}_keys");
    let dat_id = graph.create(parent, &def, Some(&dat_name)).ok()?;
    graph.node_mut_quiet(dat_id).pos = default_ramp_dat_pos(owner_pos);
    let _ = graph.set_param(dat_id, "text", Value::Str(DEFAULT_RAMP_TABLE.into()));

    let rel = format!("./{dat_name}");
    let _ = graph.set_param(owner, PARAM_DAT, Value::Str(rel));
    link_ramp_dat(graph, owner, dat_id);

    Some(dat_id)
}

pub fn link_ramp_dat(graph: &mut Graph, owner: NodeId, dat: NodeId) {
    for id in graph.walk().into_iter().filter(|&id| id != dat) {
        if graph.node(id).callback_owner == Some(owner) {
            graph.node_mut_quiet(id).callback_owner = None;
        }
    }
    graph.node_mut_quiet(dat).callback_owner = Some(owner);
}

pub fn reconcile_ramp_dat_links(graph: &mut Graph) {
    for owner in graph.walk() {
        if graph.node(owner).op_type != "rampTOP" {
            continue;
        }
        let Some(param) = graph.node(owner).param(PARAM_DAT) else {
            continue;
        };
        let path = param.value.as_str().trim().to_string();
        if path.is_empty() {
            continue;
        }
        let target = if path.starts_with('/') {
            graph.find(&path)
        } else {
            graph.find_from(owner, &path)
        };
        if let Some(dat) = target {
            link_ramp_dat(graph, owner, dat);
        }
    }
}
