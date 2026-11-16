//! Patch planning: catalogue, prompts, parse, apply.

use std::collections::BTreeMap;

use otd_core::OpRegistry;

use super::helpers::{
    first_array, json_to_value, unknown_keys, DELETE_KEYS, SET_KEYS, string_map,
};
use super::types::*;

pub fn parse_plan(value: &serde_json::Value, registry: &OpRegistry) -> Result<Plan, String> {
    let mut plan = Plan {
        notes: value
            .get("notes")
            .and_then(|n| n.as_str())
            .unwrap_or_default()
            .to_string(),
        viewer: value
            .get("viewer")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ..Default::default()
    };

    // Optional: a plan that only retunes or only deletes creates nothing, and
    // demanding an empty array for it is a rule the model has to remember for
    // no gain. The "nothing in it at all" check is at the bottom.
    let nodes = value
        .get("nodes")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default();

    for (i, entry) in nodes.iter().enumerate() {
        let op = entry
            .get("op")
            .and_then(|o| o.as_str())
            .ok_or_else(|| format!("node {i} has no `op`"))?
            .to_string();
        let def = registry
            .get(&op)
            .ok_or_else(|| format!("`{op}` is not an operator in this build"))?;

        let name = entry
            .get("name")
            .and_then(|n| n.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("node{}", i + 1));

        let pos = entry
            .get("pos")
            .and_then(|p| p.as_array())
            .map(|p| {
                let f = |i: usize| p.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                [f(0), f(1)]
            })
            .unwrap_or([i as f32 * 200.0, 0.0]);

        // Parameters are coerced to the operator's declared type rather than
        // taken as whatever JSON they arrived as, so `1` for a float
        // parameter is 1.0 and not an int that breaks the packing.
        let declared = (def.params)();
        let mut params = BTreeMap::new();
        if let Some(map) = entry.get("params").and_then(|p| p.as_object()) {
            for (key, raw) in map {
                if let Some(param) = declared.get(key.as_str()) {
                    if let Some(v) = json_to_value(raw, &param.value) {
                        params.insert(key.clone(), v);
                    }
                }
            }
        }
        let mut expressions = BTreeMap::new();
        if let Some(map) = entry.get("expressions").and_then(|p| p.as_object()) {
            for (key, raw) in map {
                if declared.contains_key(key.as_str()) {
                    if let Some(text) = raw.as_str() {
                        expressions.insert(key.clone(), text.to_string());
                    }
                }
            }
        }

        let exports = string_map(entry.get("exports"));

        plan.nodes.push(PlannedNode {
            name,
            op,
            pos,
            params,
            expressions,
            exports,
        });
    }

    if let Some(wires) = value.get("connections").and_then(|c| c.as_array()) {
        for wire in wires {
            let from = wire.get("from").and_then(|f| f.as_str());
            let to = wire.get("to").and_then(|t| t.as_str());
            if let (Some(from), Some(to)) = (from, to) {
                plan.connections.push(PlannedWire {
                    from: from.to_string(),
                    to: to.to_string(),
                    input: wire.get("input").and_then(|i| i.as_u64()).unwrap_or(0) as usize,
                });
            }
        }
    }

    if let Some(sets) = first_array(value, SET_KEYS) {
        for entry in sets {
            let Some(node) = entry.get("node").and_then(|n| n.as_str()) else {
                continue;
            };
            let params = entry
                .get("params")
                .and_then(|p| p.as_object())
                .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
                .unwrap_or_default();
            plan.sets.push(PlannedSet {
                node: node.to_string(),
                params,
                expressions: string_map(entry.get("expressions")),
                exports: string_map(entry.get("exports")),
            });
        }
    }

    if let Some(gone) = first_array(value, DELETE_KEYS) {
        for entry in gone {
            if let Some(name) = entry.as_str() {
                plan.deletes.push(name.to_string());
            }
        }
    }

    let unknown = unknown_keys(value);
    if !unknown.is_empty() {
        plan.warnings.push(format!(
            "ignored {} in the reply — not part of a plan; if that was meant to \
             change the patch, it did not",
            unknown.join(", ")
        ));
    }

    // A plan used to have to build something to be a plan. It no longer does:
    // "delete the second feedback loop" and "slow the zoom down" are complete
    // requests that create nothing, and rejecting them sent the model back to
    // invent a node it did not need.
    if plan.nodes.is_empty() && plan.sets.is_empty() && plan.deletes.is_empty() {
        return Err("the plan had nothing in it — no nodes, no changes, no deletions".into());
    }
    Ok(plan)
}

/// Confine a plan to the operators the user selected.
///
/// The counterpart to [`SCOPE_RULE`], and the reason it means something. A
/// model told "only touch these" mostly complies, and mostly is not what
/// "only change these nodes" asks for — the whole point of selecting first is
/// that the rest of the patch is known to be safe. So the plan is filtered
/// against the selection rather than trusted to have been.
///
/// Three things count as changing an existing operator, and all three are
/// dropped when it is not in scope:
///
///  * a `set` on it,
///  * a `delete` of it,
///  * a connection whose *destination* is it — wiring something new into an
///    operator's input changes what that operator does just as surely as
///    retuning it. Reading from it is left alone: `from` may name anything,
///    which is what lets a new chain hang off a locked source.
///
/// Anything the plan creates is in scope by definition; it did not exist to
/// be protected. An empty `scope` means no scoping and nothing is filtered.
pub fn gate_scope(plan: &mut Plan, scope: &[String]) -> Option<String> {
    if scope.is_empty() {
        return None;
    }
    let made: Vec<String> = plan.nodes.iter().map(|n| n.name.clone()).collect();
    let allowed = |name: &str| {
        let bare = name.trim().trim_start_matches('/');
        scope.iter().any(|s| s == bare) || made.iter().any(|m| m == bare)
    };

    let mut spared: Vec<String> = Vec::new();
    plan.sets.retain(|s| {
        allowed(&s.node) || {
            spared.push(format!("retuning {}", s.node));
            false
        }
    });
    plan.deletes.retain(|d| {
        allowed(d) || {
            spared.push(format!("removing {d}"));
            false
        }
    });
    plan.connections.retain(|w| {
        allowed(&w.to) || {
            spared.push(format!("rewiring {}", w.to));
            false
        }
    });

    if spared.is_empty() {
        return None;
    }
    spared.sort();
    spared.dedup();
    Some(format!(
        "skipped {} — only the selected operators are in scope",
        spared.join(", ")
    ))
}

