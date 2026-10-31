use otd_core::{EvalContext, Graph, NodeId, OpRegistry, Value};

use crate::ops::{self, DatCtx, EXECUTE, PANEL_EXECUTE, SCRIPT, TEXT};

pub const PARAM_CALLBACKS: &str = "callbacks";
pub const PARAM_INLINE: &str = "source";

const NODE_MAIN_H: f32 = 117.0;
const CALLBACK_FOLD_STRIP_H: f32 = 16.0;
const NETWORK_GRID_STEP: f32 = 40.0;

pub fn default_callbacks_dat_pos(owner_pos: [f32; 2]) -> [f32; 2] {
    [
        owner_pos[0],
        owner_pos[1] + NODE_MAIN_H + CALLBACK_FOLD_STRIP_H + NETWORK_GRID_STEP,
    ]
}

pub fn operator_uses_callbacks_dat(op_type: &str) -> bool {
    matches!(
        op_type,
        EXECUTE
            | "chopexecuteDAT"
            | "parameterexecuteDAT"
            | PANEL_EXECUTE
            | ops::WEBCLIENT
            | ops::WEBSERVER
            | ops::UDP_IN
            | ops::UDP_OUT
    )
}

pub fn default_inline_script(op_type: &str) -> &'static str {
    match op_type {
        EXECUTE => ops::EXECUTE_SOURCE,
        "chopexecuteDAT" => ops::CHOP_EXECUTE_SOURCE,
        "parameterexecuteDAT" => ops::PAR_EXECUTE_SOURCE,
        PANEL_EXECUTE => ops::PANEL_EXECUTE_SOURCE,
        ops::WEBCLIENT => ops::WEBCLIENT_CALLBACKS,
        ops::WEBSERVER => ops::WEBSERVER_CALLBACKS,
        ops::UDP_IN | ops::UDP_OUT => ops::UDP_RECEIVE_CALLBACKS,
        _ => "",
    }
}

pub fn resolved_callbacks_dat(graph: &Graph, owner: NodeId) -> Option<NodeId> {
    let owner_node = graph.get(owner)?;
    let stored = owner_node
        .param(PARAM_CALLBACKS)
        .map(|p| p.value.as_str())
        .unwrap_or_default()
        .to_string();
    let cb_path = stored.trim();
    if cb_path.is_empty() {
        return None;
    }
    let target = if cb_path.starts_with('/') {
        graph.find(cb_path)
    } else {
        graph.find_from(owner, cb_path)
    }?;
    let node = graph.get(target)?;
    match node.op_type.as_str() {
        TEXT | SCRIPT => Some(target),
        _ => None,
    }
}

pub fn uses_external_callbacks_dat(graph: &Graph, owner: NodeId) -> bool {
    resolved_callbacks_dat(graph, owner).is_some()
}

fn clear_inline_callback_source(graph: &mut Graph, owner: NodeId) {
    let op_type = graph.node(owner).op_type.clone();
    if !operator_uses_callbacks_dat(&op_type) {
        return;
    }
    let _ = graph.set_param(owner, PARAM_INLINE, Value::Str(String::new()));
}

fn restore_inline_callback_source(graph: &mut Graph, owner: NodeId) {
    let op_type = graph.node(owner).op_type.clone();
    if !operator_uses_callbacks_dat(&op_type) {
        return;
    }
    let current = graph
        .node(owner)
        .param(PARAM_INLINE)
        .map(|p| p.value.as_str())
        .unwrap_or_default();
    if !current.trim().is_empty() {
        return;
    }
    let body = default_inline_script(&op_type);
    let _ = graph.set_param(owner, PARAM_INLINE, Value::Str(body.into()));
}

pub fn sync_owner_callback_source(graph: &mut Graph, owner: NodeId) {
    if uses_external_callbacks_dat(graph, owner) {
        clear_inline_callback_source(graph, owner);
    } else {
        restore_inline_callback_source(graph, owner);
    }
}

pub fn resolve_callback_script(
    graph: &Graph,
    owner: NodeId,
    eval: &EvalContext,
    callbacks_param: &str,
    inline_param: &str,
) -> String {
    let Some(owner_node) = graph.get(owner) else {
        return String::new();
    };
    let cb_path = owner_node
        .param(callbacks_param)
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default();
    let cb_path = cb_path.trim();
    if !cb_path.is_empty() {
        let target = if cb_path.starts_with('/') {
            graph.find(cb_path)
        } else {
            graph.find_from(owner, cb_path)
        };
        if let Some(id) = target {
            if let Some(script) = script_from_dat_node(graph, id, eval) {
                return script;
            }
        }
    }
    owner_node
        .param(inline_param)
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default()
}

fn script_from_dat_node(graph: &Graph, id: NodeId, eval: &EvalContext) -> Option<String> {
    let node = graph.get(id)?;
    match node.op_type.as_str() {
        TEXT => node.param("text").map(|p| p.eval(eval).as_str()),
        SCRIPT => node.param("source").map(|p| p.eval(eval).as_str()),
        _ => None,
    }
}

pub fn install_callbacks_dat(
    graph: &mut Graph,
    registry: &OpRegistry,
    owner: NodeId,
) -> Option<NodeId> {
    let owner_node = graph.get(owner)?;
    if !operator_uses_callbacks_dat(&owner_node.op_type) {
        return None;
    }
    let already_set = owner_node
        .param(PARAM_CALLBACKS)
        .map(|p| !p.value.as_str().trim().is_empty())
        .unwrap_or(false);
    if already_set {
        return None;
    }
    let owner_name = owner_node.name.clone();
    let owner_pos = owner_node.pos;
    let op_type = owner_node.op_type.clone();
    let parent = owner_node.parent?;

    let def = registry.get(TEXT)?.clone();
    let cb_name = format!("{owner_name}_callbacks");
    let cb_id = graph.create(parent, &def, Some(&cb_name)).ok()?;
    graph.node_mut_quiet(cb_id).pos = default_callbacks_dat_pos(owner_pos);

    let body = default_inline_script(&op_type);
    let _ = graph.set_param(cb_id, "text", Value::Str(body.into()));

    let rel = format!("./{cb_name}");
    let _ = graph.set_param(owner, PARAM_CALLBACKS, Value::Str(rel));
    link_callbacks_dat(graph, owner, cb_id);

    Some(cb_id)
}

pub fn link_callbacks_dat(graph: &mut Graph, owner: NodeId, dat: NodeId) {
    for id in graph.walk().into_iter().filter(|&id| id != dat) {
        if graph.node(id).callback_owner == Some(owner) {
            graph.node_mut_quiet(id).callback_owner = None;
        }
    }
    graph.node_mut_quiet(dat).callback_owner = Some(owner);
    clear_inline_callback_source(graph, owner);
}

pub fn reconcile_callback_links(graph: &mut Graph) {
    let root = graph.root();
    for owner in graph.walk() {
        if owner == root {
            continue;
        }
        let op_type = graph.node(owner).op_type.clone();
        if !operator_uses_callbacks_dat(&op_type) {
            continue;
        }
        let Some(param) = graph.node(owner).param(PARAM_CALLBACKS) else {
            continue;
        };
        let path = param.value.as_str().trim().to_string();
        if path.is_empty() {
            restore_inline_callback_source(graph, owner);
            continue;
        }
        let target = if path.starts_with('/') {
            graph.find(&path)
        } else {
            graph.find_from(owner, &path)
        };
        if let Some(dat) = target {
            link_callbacks_dat(graph, owner, dat);
        }
    }
}

pub fn detach_callbacks_before_delete(graph: &mut Graph, doomed: &[NodeId]) {
    let doomed_set: std::collections::HashSet<_> = doomed.iter().copied().collect();
    for id in doomed {
        if let Some(owner) = graph.node(*id).callback_owner {
            if !doomed_set.contains(&owner) {
                let _ = graph.set_param(owner, PARAM_CALLBACKS, Value::Str(String::new()));
                restore_inline_callback_source(graph, owner);
            }
        }
    }
}

pub fn callback_dats_owned_by(graph: &Graph, owners: &[NodeId]) -> Vec<NodeId> {
    let set: std::collections::HashSet<_> = owners.iter().copied().collect();
    graph
        .walk()
        .into_iter()
        .filter(|id| {
            graph
                .node(*id)
                .callback_owner
                .is_some_and(|o| set.contains(&o))
        })
        .collect()
}

impl DatCtx<'_> {
    pub fn callback_script(&self) -> String {
        resolve_callback_script(
            self.graph,
            self.id,
            self.eval,
            PARAM_CALLBACKS,
            PARAM_INLINE,
        )
    }
}
