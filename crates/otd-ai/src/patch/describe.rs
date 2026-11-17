//! Patch planning: catalogue, prompts, parse, apply.

use std::collections::BTreeMap;

use otd_core::{Graph, NodeId, OpRegistry, Value};

use super::catalogue::short_value;

/// What the network being edited currently looks like, so the model can extend it.
pub fn describe(
    graph: &Graph,
    parent: NodeId,
    selected: Option<NodeId>,
    viewer: Option<NodeId>,
    scope: &[NodeId],
    registry: &OpRegistry,
) -> String {
    let children = graph.node(parent).children.clone();
    if children.is_empty() {
        return "The network is empty.".into();
    }
    let mut out = String::from(
        "THE NETWORK AS IT STANDS\n\
         These operators already exist and are cooking. Build on them: wire \
         into them, add after them, and change their parameters. Do not \
         rebuild what is already here.\n",
    );

    // Who reads whom through a channel expression. A CHOP with no wire out of
    // it can still be what a parameter three nodes away is driven by.
    let mut read_by: BTreeMap<NodeId, Vec<String>> = BTreeMap::new();
    for id in &children {
        let node = graph.node(*id);
        for (key, param) in &node.params {
            if param.mode != otd_core::ParamMode::Expression {
                continue;
            }
            for path in param.referenced_ops() {
                if let Some(src) = graph.find_from(*id, path) {
                    if graph.node(src).family == otd_core::Family::Chop {
                        read_by
                            .entry(src)
                            .or_default()
                            .push(format!("{}.{key}", node.name));
                    }
                }
            }
        }
    }
    let loops = loop_members(graph, &children);

    for id in children {
        let node = graph.node(id);
        out.push_str(&format!(
            "- {} ({}) at [{:.0},{:.0}]",
            node.name, node.op_type, node.pos[0], node.pos[1]
        ));

        let wired: Vec<String> = node
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.map(|src| format!("{}<-{}", i, graph.node(src).name)))
            .collect();
        if !wired.is_empty() {
            out.push_str(&format!(" inputs: {}", wired.join(" ")));
        }

        // Only what was changed. Everything else is in the catalogue.
        let defaults = registry.get(&node.op_type).map(|def| (def.params)());
        for (key, param) in &node.params {
            if !param.expression.is_empty() {
                out.push_str(&format!(" {key}=expr({})", clip(&param.expression)));
                continue;
            }
            let unchanged = defaults
                .as_ref()
                .and_then(|d| d.get(key))
                .map(|d| d.value == param.value)
                .unwrap_or(false);
            if !unchanged {
                out.push_str(&format!(" {key}={}", context_value(&param.value)));
            }
        }

        if let Some(file) = &node.external {
            out.push_str(&format!(" component={}", clip(file)));
        }
        if node.flags.bypass {
            out.push_str(" [bypassed]");
        }
        if node.flags.render {
            out.push_str(" [output]");
        }
        if let Some(readers) = read_by.get(&id) {
            out.push_str(&format!(
                " [READ BY {} through export — live, not clutter]",
                readers.join(", ")
            ));
        }
        if let Some(fb) = loops.get(&id) {
            out.push_str(&format!(
                " [IN THE FEEDBACK LOOP of {} — anything inserted here is applied every lap]",
                graph.node(*fb).name
            ));
        }
        if Some(id) == viewer {
            out.push_str(" [VIEWER — what the user is looking at; the output]");
        }
        if Some(id) == selected {
            out.push_str(" [SELECTED — \"it\" in the request means this node]");
        }
        // Marked per node rather than listed once at the top: the model has to
        // decide operator by operator whether it may touch one, and a list
        // somewhere else is a lookup it will sometimes skip.
        if scope.contains(&id) {
            out.push_str(" [IN SCOPE]");
        } else if !scope.is_empty() {
            out.push_str(" [LOCKED]");
        }
        out.push('\n');
    }
    out
}

/// Which nodes sit inside a feedback loop, and which Feedback node owns them.
///
/// The loop is the target and everything upstream of it back to the Feedback
/// node: `fb -> decay -> drift -> mix -> out`, with `fb.target = out`. A node
/// inserted anywhere on that path is applied to the picture every lap, which
/// is what a decay is for and what a hue shift is not — one degree a frame
/// is a colour wheel spinning, and a ramp multiplied in each lap kills the
/// loop wherever the ramp is dark. The model has to know which nodes those
/// are, or "make it look better" lands its colour work in the one place it
/// accumulates.
fn loop_members(graph: &Graph, children: &[NodeId]) -> BTreeMap<NodeId, NodeId> {
    let mut members = BTreeMap::new();
    for fb in children {
        let node = graph.node(*fb);
        if !node.op_type.to_lowercase().contains("feedback") {
            continue;
        }
        let target = node
            .param("target")
            .map(|p| p.value.as_str())
            .unwrap_or_default();
        let Some(mut at) = graph.find_from(*fb, target.trim()) else {
            continue;
        };
        // Walk input 0 upstream from the target until the feedback node. A
        // composite's other input is the source coming in, not the loop.
        let mut seen = Vec::new();
        for _ in 0..32 {
            if at == *fb || seen.contains(&at) {
                break;
            }
            seen.push(at);
            // Follow whichever input leads back to the feedback, preferring
            // the one that does over input 0.
            let inputs: Vec<NodeId> = graph.node(at).inputs.iter().flatten().copied().collect();
            let next = inputs
                .iter()
                .copied()
                .find(|i| reaches_feedback_node(graph, *i, *fb, 16))
                .or_else(|| inputs.first().copied());
            match next {
                Some(n) => at = n,
                None => break,
            }
        }
        if seen
            .iter()
            .any(|id| graph.node(*id).inputs.iter().flatten().any(|i| *i == *fb))
        {
            for id in seen {
                members.insert(id, *fb);
            }
        }
    }
    members
}

/// Whether `from` reaches the specific node `fb` upstream within `depth` hops.
fn reaches_feedback_node(graph: &Graph, from: NodeId, fb: NodeId, depth: usize) -> bool {
    if from == fb {
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
        .any(|i| reaches_feedback_node(graph, *i, fb, depth - 1))
}

/// A value as the model needs to read it back.
///
/// Unlike [`short_value`], which writes a catalogue of defaults, this is
/// reporting what the user actually set — so a file path or a menu choice is
/// worth its characters where a whole shader is not.
fn context_value(v: &Value) -> String {
    match v {
        Value::Str(s) | Value::Select(s) => format!("\"{}\"", clip(s)),
        other => short_value(other),
    }
}

/// Long text — a shader body, a path — kept to a line.
///
/// Truncated rather than dropped: knowing a glslTOP already has a shader is
/// what stops the model writing a second one, and the first line is usually
/// enough to say what it is.
pub(super) fn clip(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut cut = flat.char_indices().map(|(i, _)| i);
    match cut.nth(96) {
        Some(end) => format!("{}… ({} chars)", &flat[..end], flat.chars().count()),
        None => flat,
    }
}
