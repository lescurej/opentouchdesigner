use otd_core::{Family, Graph, NodeId};

pub(crate) fn output_port_count(graph: &Graph, id: NodeId) -> usize {
    if graph.node(id).family != Family::Comp {
        return 1;
    }
    let n = graph.comp_out_connectors(id).len();
    if n == 0 { 1 } else { n }
}

pub(crate) fn output_port_family(graph: &Graph, id: NodeId, index: usize) -> Family {
    if graph.node(id).family != Family::Comp {
        return graph.node(id).family;
    }
    graph
        .comp_out_connectors(id)
        .get(index)
        .map(|(_, f, _)| *f)
        .or_else(|| graph.output_family(id))
        .unwrap_or(Family::Comp)
}
