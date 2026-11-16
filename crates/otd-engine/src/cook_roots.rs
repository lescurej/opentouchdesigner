use std::collections::HashSet;

use otd_chop::ops::{INFO, PARAMETER};
use otd_core::{EvalContext, Family, Graph, NodeId};

use crate::execute;
use crate::info;

pub fn operators_watched_by_info_parameter(graph: &Graph) -> HashSet<NodeId> {
    let mut set = HashSet::new();
    for id in graph.walk() {
        let op = graph.node(id).op_type.as_str();
        if op != INFO && op != PARAMETER {
            continue;
        }
        let path = graph
            .node(id)
            .param("op")
            .map(|p| p.eval(&EvalContext::default()).as_str())
            .unwrap_or_default();
        if let Some(watch) = info::resolve_op_from(graph, id, &path) {
            set.insert(watch);
        }
    }
    set
}

fn push_root(roots: &mut Vec<NodeId>, id: NodeId) {
    if !roots.contains(&id) {
        roots.push(id);
    }
}

fn merge_roots(output: Vec<NodeId>, editor: &[NodeId]) -> Vec<NodeId> {
    let mut roots = output;
    for &id in editor {
        push_root(&mut roots, id);
    }
    roots
}

pub fn is_side_effect_op(op_type: &str) -> bool {
    if otd_chop::io::is_side_effect_root(op_type) {
        return true;
    }
    matches!(
        op_type,
        "moviefileoutTOP"
            | "recordCHOP"
            | "fileoutCHOP"
            | "serialCHOP"
            | "audiofileoutCHOP"
            | otd_chop::io::AUDIO_MOVIE
    )
}

fn node_is_output_root(
    graph: &Graph,
    id: NodeId,
    watched: &HashSet<NodeId>,
) -> bool {
    let node = graph.node(id);
    node.flags.render
        || node.flags.cook
        || execute::is_execute(&node.op_type)
        || is_side_effect_op(&node.op_type)
        || node.op_type == INFO
        || node.op_type == PARAMETER
        || watched.contains(&id)
}

fn node_is_editor_root(graph: &Graph, id: NodeId, visible: &[NodeId]) -> bool {
    let node = graph.node(id);
    node.flags.viewer && visible.contains(&id) && node.family != Family::Comp
}

pub struct OutputRootsRequest<'a> {
    pub graph: &'a Graph,
    pub perform_ui: Option<NodeId>,
    pub extra_roots: &'a [NodeId],
}

pub fn output_roots(req: OutputRootsRequest<'_>) -> Vec<NodeId> {
    let mut roots = Vec::new();
    if let Some(v) = req.perform_ui.filter(|v| req.graph.contains(*v)) {
        push_root(&mut roots, v);
    }
    for &id in req.extra_roots {
        push_root(&mut roots, id);
    }
    let watched = operators_watched_by_info_parameter(req.graph);
    for id in req.graph.walk() {
        if id == req.graph.root() {
            continue;
        }
        if node_is_output_root(req.graph, id, &watched) {
            push_root(&mut roots, id);
        }
    }
    roots
}

pub struct EditorRootsRequest<'a> {
    pub graph: &'a Graph,
    pub visible: &'a [NodeId],
    pub extra_roots: &'a [NodeId],
}

pub fn editor_roots(req: EditorRootsRequest<'_>) -> Vec<NodeId> {
    let mut roots = Vec::new();
    for &id in req.extra_roots {
        push_root(&mut roots, id);
    }
    for id in req.graph.walk() {
        if id == req.graph.root() {
            continue;
        }
        if node_is_editor_root(req.graph, id, req.visible) {
            push_root(&mut roots, id);
        }
    }
    roots
}

pub struct CookRootsRequest<'a> {
    pub graph: &'a Graph,
    pub perform_ui: Option<NodeId>,
    pub visible: &'a [NodeId],
    pub current: NodeId,
    pub output_extra_roots: &'a [NodeId],
    pub editor_extra_roots: &'a [NodeId],
    pub include_editor: bool,
}

pub fn app_cook_roots(req: CookRootsRequest<'_>) -> Vec<NodeId> {
    let output = output_roots(OutputRootsRequest {
        graph: req.graph,
        perform_ui: req.perform_ui,
        extra_roots: req.output_extra_roots,
    });
    if !req.include_editor {
        return output;
    }
    let editor = editor_roots(EditorRootsRequest {
        graph: req.graph,
        visible: req.visible,
        extra_roots: req.editor_extra_roots,
    });
    merge_roots(output, &editor)
}

pub fn editor_cook_roots(graph: &Graph, perform_ui: Option<NodeId>) -> Vec<NodeId> {
    let mut roots = Vec::new();
    if let Some(v) = perform_ui.filter(|v| graph.contains(*v)) {
        roots.push(v);
    }
    let watched = operators_watched_by_info_parameter(graph);
    for id in graph.walk() {
        if id == graph.root() {
            continue;
        }
        let node = graph.node(id);
        let wanted = node.flags.render
            || node.flags.cook
            || execute::is_execute(&node.op_type)
            || is_side_effect_op(&node.op_type)
            || (node.flags.viewer && node.family != Family::Comp)
            || node.op_type == INFO
            || node.op_type == PARAMETER
            || watched.contains(&id);
        if wanted && !roots.contains(&id) {
            roots.push(id);
        }
    }
    roots
}

#[cfg(test)]
#[path = "tests/cook_roots__tests.rs"]
mod tests;
