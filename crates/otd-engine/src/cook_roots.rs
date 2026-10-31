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
