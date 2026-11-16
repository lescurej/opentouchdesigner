use std::collections::BTreeMap;

use otd_core::{Graph, NodeId, Value};

/// Composites that close a feedback loop in a way the source cannot survive.
///
/// A Composite TOP draws input 1 over input 0, and video is opaque, so `over`
/// means input 1 wins outright. Wire the source to input 0 and the feedback
/// to input 1 — the obvious way round, and the way a model writes it — and
/// the loop is sealed against its own source: it looks correct for about a
/// second, then the decay consumes what was already circulating and the whole
/// patch fades to black.
///
/// Worth a warning rather than a silent fix, because `over` on a feedback
/// branch is occasionally what somebody means — a matte with real alpha
/// composites exactly this way. Found by rendering the frames: it passes
/// every structural check there is.
pub(crate) fn starved_loops(graph: &Graph, made: &BTreeMap<String, NodeId>) -> Vec<String> {
    let mut out = Vec::new();
    for id in made.values().copied() {
        let node = graph.node(id);
        // Found by shape rather than by name: anything with two inputs and an
        // `operation` menu offering `over` blends the same way and starves a
        // loop the same way, and a check keyed on `compositeTOP` would stop
        // being true the day a second such operator is added.
        let Some(operation_param) = node.param("operation") else {
            continue;
        };
        let offers_over = operation_param
            .menu
            .as_ref()
            .map(|m| m.iter().any(|o| o.value.eq_ignore_ascii_case("over")))
            .unwrap_or(false);
        if !offers_over || node.inputs.len() < 2 {
            continue;
        }
        let operation = operation_param.value.as_str();
        // Empty means the default, which is `over`.
        if !(operation.is_empty() || operation.eq_ignore_ascii_case("over")) {
            continue;
        }
        // `over` at less than full opacity is a crossfade — `0.85 * last
        // frame + 0.15 * this one` — which is the classic trail and lets the
        // source in every frame. Only an opaque `over` seals the loop.
        let opacity = node
            .param("opacity")
            .and_then(|p| match p.value {
                Value::Float(f) => Some(f),
                _ => None,
            })
            .unwrap_or(1.0);
        if opacity < 0.999 {
            continue;
        }
        // Input 0 has to be carrying something, or there is nothing to hide.
        if node.inputs.first().copied().flatten().is_none() {
            continue;
        }
        let Some(over) = node.inputs.get(1).copied().flatten() else {
            continue;
        };
        if reaches_feedback(graph, over, 6) {
            out.push(node.name.clone());
        }
    }
    out
}

/// Whether a Feedback TOP is within `depth` hops upstream. The decay and the
/// transform usually sit between the feedback and the composite, so this has
/// to look further than one wire.
fn reaches_feedback(graph: &Graph, from: NodeId, depth: usize) -> bool {
    if graph.node(from).op_type.to_lowercase().contains("feedback") {
        return true;
    }
    if depth == 0 {
        return false;
    }
    graph
        .node(from)
        .inputs
        .iter()
        .flatten()
        .any(|up| reaches_feedback(graph, *up, depth - 1))
}
