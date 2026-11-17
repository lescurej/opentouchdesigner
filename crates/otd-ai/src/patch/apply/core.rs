use std::collections::BTreeMap;

use otd_core::param::channel_expression;
use otd_core::{Graph, NodeId, OpRegistry, Value};

use crate::patch::helpers::{json_to_value, split_export};
use crate::patch::types::*;

use super::starved::starved_loops;
use super::splices::splices_for;

pub fn apply(
    graph: &mut Graph,
    parent: NodeId,
    registry: &OpRegistry,
    plan: &Plan,
) -> Result<(Applied, Option<NodeId>), String> {
    let mut applied = Applied {
        warnings: plan.warnings.clone(),
        ..Default::default()
    };
    // Plan name -> the node that ended up being created for it. The graph
    // renames on collision, so this cannot be a lookup by name afterwards.
    let mut made: BTreeMap<String, NodeId> = BTreeMap::new();

    for node in &plan.nodes {
        let Some(def) = registry.get(&node.op) else {
            applied.warnings.push(format!("no operator `{}`", node.op));
            continue;
        };
        let id = match graph.create(parent, &def.clone(), Some(&node.name)) {
            Ok(id) => id,
            Err(e) => {
                applied
                    .warnings
                    .push(format!("could not create {}: {e}", node.name));
                continue;
            }
        };
        graph.node_mut_quiet(id).pos = node.pos;
        made.insert(node.name.clone(), id);
        applied.created.push(graph.node(id).name.clone());

        for (key, value) in &node.params {
            if let Err(e) = graph.set_param(id, key, value.clone()) {
                applied
                    .warnings
                    .push(format!("{}.{key}: {e}", graph.node(id).name));
            }
        }
        for (key, source) in &node.expressions {
            if let Err(e) = graph.set_expression(id, key, source) {
                applied
                    .warnings
                    .push(format!("{}.{key}: {e}", graph.node(id).name));
            }
        }
    }

    let resolve = |graph: &Graph, made: &BTreeMap<String, NodeId>, name: &str| -> Option<NodeId> {
        made.get(name).copied().or_else(|| {
            // A plan may wire into what was already there, by name or path.
            graph
                .find_from(parent, name)
                .or_else(|| graph.find_from(parent, name.trim_start_matches('/')))
        })
    };

    for wire in &plan.connections {
        let (Some(from), Some(to)) = (
            resolve(graph, &made, &wire.from),
            resolve(graph, &made, &wire.to),
        ) else {
            applied
                .warnings
                .push(format!("no node named {} or {}", wire.from, wire.to));
            continue;
        };
        match graph.connect(from, to, wire.input) {
            Ok(()) => applied.wired += 1,
            Err(e) => applied
                .warnings
                .push(format!("{} -> {}: {e}", wire.from, wire.to)),
        }
    }

    // Parameters that name another operator hold a *path*, and the model
    // wrote them before any node had one — and before the graph renamed
    // anything that collided. Rewrite bare names into real paths.
    //
    // Every path-reference parameter, not just a Feedback TOP's `target`,
    // which is all this used to cover. The others are `renderTOP.camera`,
    // `.light` and `.geometry`, `geometryCOMP.sop`, `.material` and
    // `.instancechop`, `selectTOP.top`, `replicatorCOMP.master` — which is
    // to say the entire 3D and instancing half of the program. A plan that
    // built a torus, a camera and a light wired nothing up wrong; it just
    // pointed the Render TOP at `cam1` when the node had become `/cam1`, and
    // rendered an empty frame. `is_path_ref` is the registry's own marker for
    // these, so operators added later are covered without a list here.
    //
    // Plus the ones that are deliberately *not* path refs: a Feedback
    // operator's `target` and a Select TOP's `top`. A path ref is a cook
    // dependency, and a Feedback operator that depends on its own target is
    // the cycle the previous-frame read exists to avoid — so they hold a name and the
    // engine resolves it relatively. That is fine until the graph renames on
    // collision: a plan that adds `out1` to a network that already has one
    // gets `out2`, and a `target` still saying `out1` points at the old
    // null, which reads the raw clip and the loop never closes. The `is_path_ref`
    // refactor dropped the rewrite these two used to get; this puts it back.
    let touched: Vec<NodeId> = made.values().copied().collect();
    for id in touched {
        let op_type = graph.node(id).op_type.clone();
        let by_name = |key: &str| {
            ((op_type == "feedbackTOP" || op_type == "feedbackCHOP") && key == "target")
                || (op_type == "selectTOP" && key == "top")
        };
        let refs: Vec<(String, String)> = graph
            .node(id)
            .params
            .iter()
            .filter(|(key, p)| p.is_path_ref() || by_name(key))
            .map(|(key, p)| (key.clone(), p.value.as_str()))
            .collect();
        for (key, value) in refs {
            let bare = value.trim().trim_start_matches('/');
            if bare.is_empty() {
                continue;
            }
            if let Some(node) = made.get(bare) {
                let path = graph.path(*node);
                let _ = graph.set_param(id, &key, Value::Str(path));
            }
        }
    }

    // ---- point parameters at CHOP channels
    //
    // After the wiring, and after every node exists, because an export names
    // an operator by path and the whole CHOP chain it reads from is usually
    // being created by this same plan.
    let mut export_targets: Vec<(NodeId, String, String)> = Vec::new();
    for node in &plan.nodes {
        if let Some(id) = made.get(&node.name) {
            for (key, target) in &node.exports {
                export_targets.push((*id, key.clone(), target.clone()));
            }
        }
    }
    for change in &plan.sets {
        if let Some(id) = resolve(graph, &made, &change.node) {
            for (key, target) in &change.exports {
                export_targets.push((id, key.clone(), target.clone()));
            }
        }
    }
    for (id, key, target) in export_targets {
        let Some((source, channel)) = split_export(&target) else {
            applied.warnings.push(format!(
                "{}.{key}: `{target}` is not `node:channel`",
                graph.node(id).name
            ));
            continue;
        };
        let Some(source_id) = resolve(graph, &made, source) else {
            applied.warnings.push(format!(
                "{}.{key}: no operator named {source} to export from",
                graph.node(id).name
            ));
            continue;
        };
        // Export reads channels, and only a CHOP has any. Pointing a
        // parameter at a TOP silently reads nothing and leaves a node that
        // does not move with no clue why — the same class of failure as the
        // black shader, so it is reported rather than accepted.
        if graph.node(source_id).family != otd_core::Family::Chop {
            applied.warnings.push(format!(
                "{}.{key}: {source} is a {} — only a CHOP has channels to export",
                graph.node(id).name,
                graph.node(source_id).family.suffix()
            ));
            continue;
        }
        let path = graph.path(source_id);
        match graph.set_expression(id, &key, &channel_expression(&path, channel)) {
            Ok(()) => applied
                .changed
                .push(format!("{}.{key}", graph.node(id).name)),
            Err(e) => applied
                .warnings
                .push(format!("{}.{key}: {e}", graph.node(id).name)),
        }
    }

    // ---- retune what already exists
    //
    // After the wiring, so a plan can create a node, join it on and set a
    // number on the operator it now feeds, all in one answer.
    for change in &plan.sets {
        let Some(id) = resolve(graph, &made, &change.node) else {
            applied
                .warnings
                .push(format!("no node named {} to change", change.node));
            continue;
        };
        for (key, raw) in &change.params {
            // The declared type comes off the node, which is the only place
            // it exists — `set` names an operator the model did not create
            // and whose type it is only asserting.
            let Some(declared) = graph.node(id).param(key).map(|p| p.value.clone()) else {
                applied
                    .warnings
                    .push(format!("{} has no parameter `{key}`", change.node));
                continue;
            };
            let Some(value) = json_to_value(raw, &declared) else {
                applied.warnings.push(format!(
                    "{}.{key}: {raw} is not a {declared:?}",
                    change.node
                ));
                continue;
            };
            match graph.set_param(id, key, value) {
                Ok(()) => applied.changed.push(format!("{}.{key}", change.node)),
                Err(e) => applied.warnings.push(format!("{}.{key}: {e}", change.node)),
            }
        }
        for (key, source) in &change.expressions {
            if graph.node(id).param(key).is_none() {
                applied
                    .warnings
                    .push(format!("{} has no parameter `{key}`", change.node));
                continue;
            }
            match graph.set_expression(id, key, source) {
                Ok(()) => applied.changed.push(format!("{}.{key}", change.node)),
                Err(e) => applied.warnings.push(format!("{}.{key}: {e}", change.node)),
            }
        }
    }

    // ---- remove what was asked to go
    //
    // Last, so everything above could rewire around it first, and fenced in
    // three ways. A model that has just been handed deletion is a model that
    // can empty somebody's patch on a misread, and none of these cost
    // anything when it has read correctly:
    //
    //  * **Direct children of this network only.** `resolve` will happily
    //    find a path into a component the user is not looking at; a delete
    //    that reaches somewhere off screen is not one they can see happen.
    //  * **Nothing it created this turn.** Creating a node and deleting it in
    //    the same answer is a model talking to itself, and honouring it makes
    //    the reported node count a lie.
    //  * **Never the whole network.** Clearing the canvas is `/clear`, one
    //    word, undoable and obviously deliberate. Arriving at it through
    //    "make it simpler" is not what anybody meant.
    let children: Vec<NodeId> = graph.node(parent).children.clone();
    let mut doomed: Vec<NodeId> = Vec::new();
    for name in &plan.deletes {
        let bare = name.trim().trim_start_matches('/');
        if made.contains_key(bare) {
            applied
                .warnings
                .push(format!("{bare} was created by this plan; not deleting it"));
            continue;
        }
        match graph
            .find_from(parent, bare)
            .filter(|id| children.contains(id))
        {
            Some(id) if !doomed.contains(&id) => doomed.push(id),
            Some(_) => {}
            None => applied
                .warnings
                .push(format!("no node named {bare} in this network to delete")),
        }
    }
    for id in graph.owned_callback_dats(&doomed) {
        if !doomed.contains(&id) {
            doomed.push(id);
        }
    }
    let survivors = children.len() + made.len() - doomed.len();
    if !doomed.is_empty() && survivors == 0 {
        applied.warnings.push(format!(
            "refused to delete all {} operators — use /clear if that is what you meant",
            doomed.len()
        ));
        doomed.clear();
    }
    // Every splice is worked out against the graph as it stands, then all of
    // them are removed, then the wires are run. Healing one node at a time
    // reads a graph the previous removal already changed: deleting `b` and
    // `c` out of `a -> b -> c -> d` would look up `c`'s input *after* `b` was
    // gone, find nothing, and leave `d` with an empty input.
    let splices = splices_for(graph, &doomed);
    otd_dat::callbacks::detach_callbacks_before_delete(graph, &doomed);
    let remove_roots = doomed.clone();
    for id in &remove_roots {
        applied.removed.push(graph.node(*id).name.clone());
        let _ = graph.remove(*id);
    }
    for (src, consumer, slot) in splices {
        let _ = graph.connect(src, consumer, slot);
    }

    for name in starved_loops(graph, &made) {
        applied.warnings.push(format!(
            "{name} closes a feedback loop with `over`, which hides its other \
             input — the picture will fade to black. Use add, screen or maximum."
        ));
    }

    let viewer = plan
        .viewer
        .as_ref()
        .and_then(|name| resolve(graph, &made, name));
    Ok((applied, viewer))
}
