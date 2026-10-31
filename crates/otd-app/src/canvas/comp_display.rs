use otd_core::{Family, Graph, NodeId};

pub fn comp_view_top(graph: &Graph, comp: NodeId) -> Option<NodeId> {
    if graph.node(comp).family != Family::Comp {
        return None;
    }
    let op_type = graph.node(comp).op_type.as_str();
    if op_type == otd_engine::panel::OP_VIEWER {
        let path = graph
            .node(comp)
            .param("op")
            .map(|p| p.eval(&otd_core::EvalContext::default()).as_str())
            .unwrap_or_default();
        let path = path.trim();
        if path.is_empty() {
            return None;
        }
        let id = graph.find(path)?;
        return match graph.node(id).family {
            Family::Top => Some(id),
            Family::Comp => comp_view_top(graph, id),
            _ => graph.resolve_output(id),
        };
    }
    if matches!(
        op_type,
        otd_engine::panel::BUTTON
            | otd_engine::panel::SLIDER
            | otd_engine::panel::FIELD
    ) {
        if let Some(view) = panel_view_top(graph, comp) {
            return Some(view);
        }
    }
    if let Some(out) = graph
        .connector_ops(comp, otd_core::graph::Connector::Out)
        .into_iter()
        .find(|id| graph.node(*id).family == Family::Top)
    {
        return Some(out);
    }
    if matches!(
        op_type,
        otd_engine::panel::BUTTON
            | otd_engine::panel::SLIDER
            | otd_engine::panel::FIELD
            | otd_engine::panel::TEXT
    ) {
        if op_type == otd_engine::panel::TEXT {
            return graph
                .children(comp)
                .iter()
                .find(|id| graph.node(**id).family == Family::Top)
                .copied()
                .or_else(|| comp_out_top(graph, comp));
        }
        if let Some(text_comp) = graph.children(comp).iter().find(|id| {
            let n = graph.node(**id);
            n.name == "text" && n.op_type == otd_engine::panel::TEXT
        }) {
            return comp_view_top(graph, *text_comp);
        }
    }
    comp_out_top(graph, comp)
}

fn panel_view_top(graph: &Graph, comp: NodeId) -> Option<NodeId> {
    graph
        .children(comp)
        .iter()
        .find(|id| graph.node(**id).name == "view1" && graph.node(**id).family == Family::Top)
        .copied()
}

pub fn comp_out_top(graph: &Graph, comp: NodeId) -> Option<NodeId> {
    if graph.node(comp).family != Family::Comp {
        return None;
    }
    graph
        .connector_ops(comp, otd_core::graph::Connector::Out)
        .into_iter()
        .find(|id| graph.node(*id).family == Family::Top)
        .or_else(|| {
            graph
                .connector_ops(comp, otd_core::graph::Connector::Out)
                .into_iter()
                .find_map(|out| graph.resolve_output(out))
        })
        .or_else(|| {
            graph
                .children(comp)
                .iter()
                .find(|&&id| graph.node(id).family == Family::Top)
                .copied()
        })
}

pub fn background_display_nodes(graph: &Graph, network: NodeId) -> Vec<NodeId> {
    if !graph.contains(network) {
        return Vec::new();
    }
    let mut ids: Vec<NodeId> = graph
        .children(network)
        .iter()
        .copied()
        .filter(|id| {
            super::flags::display_flag_allowed(graph.node(*id).family) && graph.node(*id).flags.bg_display
        })
        .collect();
    ids.sort_by_key(|id| graph.path(*id));
    ids
}

pub fn append_network_display_roots(
    graph: &Graph,
    network: NodeId,
    visible: &[NodeId],
    roots: &mut Vec<NodeId>,
) {
    if !graph.contains(network) {
        return;
    }
    for id in background_display_nodes(graph, network) {
        if !roots.contains(&id) {
            roots.push(id);
        }
    }
    for &id in graph.children(network) {
        if graph.node(id).family != Family::Comp || !graph.node(id).flags.viewer {
            continue;
        }
        if !visible.contains(&id) {
            continue;
        }
        append_panel_comp_cook_roots(graph, id, roots);
    }
}

pub fn append_panel_comp_cook_roots(graph: &Graph, comp: NodeId, roots: &mut Vec<NodeId>) {
    if graph.node(comp).family != Family::Comp {
        return;
    }
    if let Some(top) = comp_view_top(graph, comp) {
        if !roots.contains(&top) {
            roots.push(top);
        }
    }
    for w in panel_widgets_under(graph, comp) {
        if w.id == comp {
            continue;
        }
        if let Some(top) = comp_view_top(graph, w.id) {
            if !roots.contains(&top) {
                roots.push(top);
            }
        }
    }
}

pub fn is_descendant_of(graph: &Graph, ancestor: NodeId, id: NodeId) -> bool {
    if id == ancestor {
        return true;
    }
    let mut cur = graph.get(id).and_then(|n| n.parent);
    while let Some(c) = cur {
        if c == ancestor {
            return true;
        }
        cur = graph.get(c).and_then(|n| n.parent);
    }
    false
}

pub fn panel_widgets_under(
    graph: &Graph,
    comp: NodeId,
) -> Vec<otd_engine::panel::Widget> {
    otd_engine::panel::widgets(graph)
        .into_iter()
        .filter(|w| is_descendant_of(graph, comp, w.id))
        .collect()
}

pub fn append_perform_panel_cook_roots(graph: &Graph, perform_comp: NodeId, roots: &mut Vec<NodeId>) {
    append_panel_comp_cook_roots(graph, perform_comp, roots);
}

pub fn panel_widgets_local_to_comp(
    graph: &Graph,
    comp: NodeId,
) -> Vec<otd_engine::panel::Widget> {
    otd_engine::panel::panel_direct_display_widgets(graph, comp)
}
