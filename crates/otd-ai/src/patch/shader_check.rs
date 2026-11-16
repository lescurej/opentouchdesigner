//! Patch planning: catalogue, prompts, parse, apply.


use otd_core::{Graph, NodeId, Value};

use super::types::*;
// ------------------------------------------------------------ shader checks

/// A shader compiler, supplied by the caller.
///
/// `otd-ai` must not depend on the GPU crate — the whole point of it having
/// no GPU dependency is that key handling and validation are testable without
/// one — so the compiler arrives as a function pointer instead. The editor
/// passes naga's front end, which is the same one that would reject the
/// shader a moment later on the node.
pub type ShaderCheck = fn(source: &str, is_glsl: bool) -> Result<(), String>;

/// The operators whose `source` parameter holds a shader, lowercased.
///
/// `source` is not a shader-only parameter name — the script DATs keep their
/// Python in one — and a model will occasionally hang a stray `source` on an
/// operator that has no such parameter at all. Neither is WGSL, and neither
/// should be compiled as if it were.
const SHADER_OPS: &[&str] = &["glsltop"];

/// Every shader in a reply that will not compile, as `(node, error)`.
///
/// Worth doing *before* the nodes exist. A model that writes a shader
/// referencing something it invented produces a node that outlines red and an
/// output that is silently black — the patch looks built and is not.
///
/// Keyed on the node's operator, not on it having a `source`: a stray `source`
/// on a levelTOP is a parameter [`apply`] drops on the floor, and compiling it
/// as WGSL buys nothing but a repair round trip the model cannot act on.
pub fn shader_problems(value: &serde_json::Value, check: ShaderCheck) -> Vec<(String, String)> {
    let mut problems = Vec::new();
    let Some(nodes) = value.get("nodes").and_then(|n| n.as_array()) else {
        return problems;
    };
    for (i, node) in nodes.iter().enumerate() {
        let is_shader = node
            .get("op")
            .and_then(|o| o.as_str())
            .map(|op| SHADER_OPS.contains(&op.to_lowercase().as_str()))
            .unwrap_or(false);
        if !is_shader {
            continue;
        }
        let Some(params) = node.get("params").and_then(|p| p.as_object()) else {
            continue;
        };
        let Some(source) = params.get("source").and_then(|s| s.as_str()) else {
            continue;
        };
        if source.trim().is_empty() {
            continue;
        }
        let is_glsl = params
            .get("language")
            .and_then(|l| l.as_str())
            .map(|l| l.eq_ignore_ascii_case("glsl"))
            .unwrap_or(false);
        if let Err(e) = check(source, is_glsl) {
            let name = node
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("node{}", i + 1));
            problems.push((name, e));
        }
    }
    problems
}

/// Path parameters on the named nodes that point at nothing, as
/// `node.key=value`.
///
/// No warning covers this and the symptom is a black texture: a Render TOP
/// whose `camera` is a name that never became a path renders an empty frame
/// and reports nothing. The scenario and vfx harnesses print it as
/// `DANGLING REF`; this is the same check for anything else that applies a
/// plan.
pub fn unresolved_refs(graph: &Graph, parent: NodeId, names: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        let Some(id) = graph.find_from(parent, name) else {
            continue;
        };
        for (key, p) in &graph.node(id).params {
            if !p.is_path_ref() {
                continue;
            }
            let v = p.value.as_str();
            if !v.trim().is_empty() && graph.find_from(id, v.trim()).is_none() {
                out.push(format!("{name}.{key}={v}"));
            }
        }
    }
    out
}

/// Nodes a plan creates that nothing reads and nothing looks at.
///
/// A model asked to add to a patch will sometimes build a whole second chain
/// and forget to join it on. The nodes are real, they cook, and they do
/// nothing — worth saying so rather than leaving them to be found.
pub fn dangling(plan: &Plan) -> Vec<String> {
    // Not everything is read by a wire. A Render TOP reads its camera, light
    // and geometry by name in a parameter; a Geometry COMP reads its SOP and
    // MAT the same way; an export reads a CHOP as `node:channel`. Counting
    // only wires called every camera in every 3D plan "wired to nothing",
    // which is a warning about the one part of the patch that was right.
    let named: Vec<&str> = plan
        .nodes
        .iter()
        .flat_map(|n| {
            n.params
                .values()
                .filter_map(|v| match v {
                    Value::Str(s) => Some(s.trim().trim_start_matches('/')),
                    _ => None,
                })
                .chain(n.exports.values().filter_map(|e| e.split(':').next()))
        })
        .collect();
    plan.nodes
        .iter()
        .filter(|node| {
            let read = plan.connections.iter().any(|w| w.from == node.name)
                || named.contains(&node.name.as_str());
            let viewed = plan.viewer.as_deref() == Some(node.name.as_str());
            // A node that feeds nothing and is not the output. Feedback is
            // exempt: it is read by its `target`, not by a wire.
            !read && !viewed && !node.op.to_lowercase().contains("feedback")
        })
        .map(|node| node.name.clone())
        .collect()
}
