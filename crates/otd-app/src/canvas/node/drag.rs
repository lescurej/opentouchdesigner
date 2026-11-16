use otd_core::NodeId;

use crate::app::OtdApp;

pub(crate) fn node_drag_group(app: &OtdApp, primary: NodeId) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = if app.is_selected(primary) && app.canvas.selection.len() > 1 {
        app.canvas.selection
            .iter()
            .copied()
            .filter(|n| app.graph.contains(*n) && !app.graph.node(*n).flags.lock)
            .collect()
    } else {
        vec![primary]
    };
    if !ids.contains(&primary) {
        ids.push(primary);
    }
    append_owned_callback_dats(app, &mut ids);
    append_docked_nodes(app, &mut ids);
    ids.sort_by_key(|n| app.graph.path(*n));
    ids
}

fn append_docked_nodes(app: &OtdApp, ids: &mut Vec<NodeId>) {
    let anchors: Vec<NodeId> = ids.clone();
    let mut extra = Vec::new();
    for anchor in anchors {
        let owner = app.graph.node(anchor).docked_to.unwrap_or(anchor);
        if app.graph.contains(owner) {
            extra.push(owner);
        }
        for (other, node) in app.graph.iter() {
            if node.docked_to == Some(owner) {
                extra.push(other);
            }
        }
    }
    for id in extra {
        if app.graph.contains(id) && !app.graph.node(id).flags.lock && !ids.contains(&id) {
            ids.push(id);
        }
    }
}

fn append_owned_callback_dats(app: &OtdApp, ids: &mut Vec<NodeId>) {
    let anchors: Vec<NodeId> = ids.clone();
    for anchor in anchors {
        let Some(cb) = app.graph.owned_callback_dat(anchor) else {
            continue;
        };
        if !app.graph.contains(cb) || app.graph.node(cb).flags.lock {
            continue;
        }
        if !ids.contains(&cb) {
            ids.push(cb);
        }
    }
}

pub(crate) fn node_drag_snapshot(app: &OtdApp, id: NodeId) -> Vec<(NodeId, [f32; 2])> {
    node_drag_group(app, id)
        .into_iter()
        .map(|n| (n, app.graph.node(n).pos))
        .collect()
}
