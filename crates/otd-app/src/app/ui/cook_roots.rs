use super::*;

/// What has to cook this frame: perform output, render/cook roots, execute
/// DATs, and every node whose viewer flag is on and on screen.
///
/// That last condition is what makes the demand-driven engine visible: pan a
/// heavy branch off the canvas and it stops costing anything. It is also what
/// makes perform mode cheap rather than merely dark — nothing is on the canvas
/// there, so `visible` is empty and the only thing left is the output chain.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn cook_roots(
    graph: &Graph,
    viewer: Option<NodeId>,
    visible: &[NodeId],
    current: NodeId,
) -> Vec<NodeId> {
    let mut roots = Vec::new();
    if let Some(v) = viewer.filter(|v| graph.contains(*v)) {
        roots.push(v);
    }
    crate::canvas::comp_display::append_network_display_roots(graph, current, visible, &mut roots);
    for id in graph.walk() {
        if id == graph.root() {
            continue;
        }
        let node = graph.node(id);
        let wanted = node.flags.render
            || node.flags.cook
            || otd_engine::execute::is_execute(&node.op_type)
            || (node.flags.viewer && visible.contains(&id) && node.family != otd_core::Family::Comp);
        if wanted && !roots.contains(&id) {
            roots.push(id);
        }
    }
    roots
}
