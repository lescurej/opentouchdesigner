//! Unknown operator types from older projects become family placeholders
//! (`missingCHOP`, …) so a patch still opens and shows what is wrong.

use crate::OpDef;
use crate::graph::{Graph, Node, NodeId, OpRegistry};
use crate::indexmap::IndexMap;
use crate::param::Param;
use crate::value::Value;

pub const ORIGINAL_TYPE_PARAM: &str = "originaltype";

pub const MISSING_CHOP: &str = "missingCHOP";
pub const MISSING_TOP: &str = "missingTOP";
pub const MISSING_SOP: &str = "missingSOP";
pub const MISSING_DAT: &str = "missingDAT";
pub const MISSING_COMP: &str = "missingCOMP";

pub fn missing_placeholder_type(op: &str) -> &'static str {
    if op.ends_with("CHOP") {
        MISSING_CHOP
    } else if op.ends_with("TOP") {
        MISSING_TOP
    } else if op.ends_with("SOP") {
        MISSING_SOP
    } else if op.ends_with("DAT") {
        MISSING_DAT
    } else if op.ends_with("COMP") {
        MISSING_COMP
    } else {
        MISSING_CHOP
    }
}

pub fn is_missing_placeholder(op: &str) -> bool {
    matches!(
        op,
        MISSING_CHOP | MISSING_TOP | MISSING_SOP | MISSING_DAT | MISSING_COMP
    )
}

pub fn display_op_type(node: &Node) -> String {
    if is_missing_placeholder(&node.op_type) {
        if let Some(p) = node.param(ORIGINAL_TYPE_PARAM) {
            let s = p.value.as_str();
            if !s.is_empty() {
                return s;
            }
        }
    }
    node.op_type.clone()
}

pub fn node_entry_op_type(node: &Node) -> String {
    display_op_type(node)
}

pub fn missing_params() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        ORIGINAL_TYPE_PARAM.into(),
        Param::str("").with_label("Missing Operator Type"),
    );
    m
}

pub struct ResolvedOpDef<'a> {
    pub def: &'a OpDef,
    /// Set when the project named an operator this build does not register.
    pub substituted: bool,
    pub original_type: String,
}

pub fn resolve_op_def<'a>(registry: &'a OpRegistry, op: &str) -> Option<ResolvedOpDef<'a>> {
    if let Some(def) = registry.get(op) {
        return Some(ResolvedOpDef {
            def,
            substituted: false,
            original_type: op.to_string(),
        });
    }
    let placeholder = missing_placeholder_type(op);
    let def = registry.get(placeholder)?;
    Some(ResolvedOpDef {
        def,
        substituted: true,
        original_type: op.to_string(),
    })
}

pub fn finish_missing_substitution(graph: &mut Graph, id: NodeId, resolved: &ResolvedOpDef<'_>) {
    if !resolved.substituted {
        return;
    }
    let _ = graph.set_param(
        id,
        ORIGINAL_TYPE_PARAM,
        Value::Str(resolved.original_type.clone()),
    );
    graph.load_warnings.push(format!(
        "{}: unknown operator `{}` (placeholder {})",
        graph.path(id),
        resolved.original_type,
        graph.node(id).op_type
    ));
}

pub fn missing_node_message(graph: &Graph, id: NodeId) -> Option<String> {
    let node = graph.get(id)?;
    if !is_missing_placeholder(&node.op_type) {
        return None;
    }
    let orig = display_op_type(node);
    Some(format!("Unknown operator: {orig}"))
}
