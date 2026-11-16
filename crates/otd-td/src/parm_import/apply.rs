use otd_core::graph::{Graph, NodeId};
use otd_core::param::ParamMode;
use otd_core::value::Value;

use super::mapping::{
    coerce_for_param, map_parm_key, map_td_value, param_for_td_value, td_mode_uses_expression,
};
use super::parse::ParsedParm;

pub(crate) fn apply_one_parm(
    graph: &mut Graph,
    id: NodeId,
    key: &str,
    td_mode: u32,
    constant: Option<Value>,
    expression: Option<&str>,
) {
    let expression_mode = td_mode_uses_expression(td_mode) && expression.is_some();
    if expression_mode {
        let node = graph.node_mut(id);
        if let Some(param) = node.params.get_mut(key) {
            if let Some(ref c) = constant {
                param.value = coerce_for_param(param, c.clone());
            }
            param.mode = ParamMode::Expression;
            param.expression = expression.unwrap().to_string();
            param.recompile();
        }
        return;
    }
    if let Some(val) = constant {
        let node = graph.node_mut(id);
        if let Some(param) = node.params.get_mut(key) {
            let coerced = coerce_for_param(param, val);
            param.value = coerced.clone();
            param.mode = ParamMode::Constant;
            param.expression.clear();
            param.recompile();
        } else {
            let _ = node;
            let _ = graph.set_param(id, key, val);
        }
    }
}

pub fn apply_parsed_parms(
    graph: &mut Graph,
    id: NodeId,
    otd_op: &str,
    parsed: &[ParsedParm],
    warnings: &mut Vec<String>,
) {
    let _ = warnings;
    if otd_op == "constantCHOP" {
        let has_row = parsed
            .iter()
            .any(|p| p.key.starts_with("const") && p.key.contains("name"));
        if has_row && graph.node(id).params.contains_key("rows") {
            let _ = graph.set_param(id, "rows", Value::Int(1));
        }
    }

    let mut phong_diff = [0.0f64, 0.0, 0.0];
    let mut phong_diff_set = false;
    let mut phong_spec = 0.0f64;
    let mut phong_spec_set = false;
    let mut phong_emit = 0.0f64;
    let mut phong_emit_set = false;
    let mut constant_color = [1.0f64, 1.0, 1.0];
    let mut constant_color_set = false;

    for p in parsed {
        let td_key = p.key.as_str();
        let key = map_parm_key(otd_op, td_key);
        let constant = p.constant.clone().map(|v| map_td_value(otd_op, td_key, v));

        if otd_op == "constantMAT" && matches!(td_key, "colorr" | "colorg" | "colorb") {
            if let Some(Value::Float(v)) = &constant {
                constant_color_set = true;
                match td_key {
                    "colorr" => constant_color[0] = *v,
                    "colorg" => constant_color[1] = *v,
                    "colorb" => constant_color[2] = *v,
                    _ => {}
                }
            }
            continue;
        }
        if otd_op == "phongMAT" && matches!(td_key, "diffr" | "diffg" | "diffb") {
            if let Some(Value::Float(v)) = &constant {
                phong_diff_set = true;
                match td_key {
                    "diffr" => phong_diff[0] = *v,
                    "diffg" => phong_diff[1] = *v,
                    "diffb" => phong_diff[2] = *v,
                    _ => {}
                }
            }
            continue;
        }
        if otd_op == "phongMAT" && matches!(td_key, "specr" | "specg" | "specb") {
            if let Some(Value::Float(v)) = &constant {
                phong_spec_set = true;
                phong_spec = phong_spec.max(*v);
            }
            continue;
        }
        if otd_op == "phongMAT" && matches!(td_key, "emitr" | "emitg" | "emitb") {
            if let Some(Value::Float(v)) = &constant {
                phong_emit_set = true;
                phong_emit = phong_emit.max(*v);
            }
            continue;
        }
        if otd_op == "constantMAT" && matches!(td_key, "emitr" | "emitg" | "emitb") {
            if let Some(Value::Float(v)) = &constant {
                let _ = graph.set_param(id, "emit", Value::Float(*v));
            }
            continue;
        }
        if otd_op == "phongMAT" && td_key == "shininess" {
            if let Some(Value::Float(v)) = constant {
                let _ = graph.set_param(id, "shininess", Value::Float(v));
            }
            continue;
        }

        if graph.node(id).params.contains_key(&key) {
            apply_one_parm(
                graph,
                id,
                &key,
                p.td_mode,
                constant,
                p.expression.as_deref(),
            );
        } else if let Some(ref val) = constant {
            let mut param = param_for_td_value(val);
            if td_mode_uses_expression(p.td_mode) {
                if let Some(expr) = &p.expression {
                    param.mode = ParamMode::Expression;
                    param.expression = expr.clone();
                    param.recompile();
                }
            }
            graph.add_custom_param(id, &key, param);
        }
    }

    if constant_color_set && graph.node(id).params.contains_key("basecolor") {
        let _ = graph.set_param(
            id,
            "basecolor",
            Value::Vec4([constant_color[0], constant_color[1], constant_color[2], 1.0]),
        );
    }

    if phong_diff_set && graph.node(id).params.contains_key("basecolor") {
        let a = graph
            .node(id)
            .params
            .get("basecolor")
            .map(|p| p.value.as_vec4_f32()[3] as f64)
            .unwrap_or(1.0);
        let _ = graph.set_param(
            id,
            "basecolor",
            Value::Vec4([phong_diff[0], phong_diff[1], phong_diff[2], a]),
        );
    }
    if phong_spec_set && graph.node(id).params.contains_key("specular") {
        let _ = graph.set_param(id, "specular", Value::Float(phong_spec));
    }
    if phong_emit_set && graph.node(id).params.contains_key("emit") {
        let _ = graph.set_param(id, "emit", Value::Float(phong_emit));
    }
}