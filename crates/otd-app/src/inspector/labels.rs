use otd_core::Family;
use otd_core::NodeId;

use crate::app::OtdApp;

pub fn op_display_label(app: &OtdApp, id: NodeId) -> String {
    let op_type = &app.graph.node(id).op_type;
    app.registry
        .get(op_type)
        .map(|d| d.label.to_string())
        .unwrap_or_else(|| op_type.clone())
}

pub fn is_python_source(app: &OtdApp, id: NodeId) -> bool {
    if otd_dat::callbacks::uses_external_callbacks_dat(&app.graph, id) {
        return false;
    }
    app.graph
        .node(id)
        .params
        .get("source")
        .is_some_and(|p| p.is_script())
}

pub fn is_shader_source(app: &OtdApp, id: NodeId) -> bool {
    if app.graph.node(id).family != Family::Top {
        return false;
    }
    let op = &app.graph.node(id).op_type;
    op == "glslTOP"
        || app
            .graph
            .node(id)
            .params
            .get("language")
            .map(|p| p.value.as_str() == "glsl")
            .unwrap_or(op == "glslTOP")
}