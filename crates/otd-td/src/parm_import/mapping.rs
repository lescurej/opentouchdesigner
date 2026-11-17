//! TouchDesigner `.parm` blocks → OTD parameter values.

use otd_core::param::Param;
use otd_core::value::Value;

pub fn parm_target_keys(otd_op: &str, td_key: &str) -> Vec<String> {
    match (otd_op, td_key) {
        ("constantMAT", "colorr" | "colorg" | "colorb") => vec!["basecolor".into()],
        ("phongMAT", "diffr" | "diffg" | "diffb") => vec!["basecolor".into()],
        ("phongMAT", "specr" | "specg" | "specb") => vec!["specular".into()],
        ("phongMAT", "emitr" | "emitg" | "emitb") => vec!["emit".into()],
        ("constantMAT", "emitr" | "emitg" | "emitb") => vec!["emit".into()],
        _ => vec![map_parm_key(otd_op, td_key)],
    }
}

pub fn map_parm_key(otd_op: &str, td_key: &str) -> String {
    match (otd_op, td_key) {
        ("chopexecuteDAT", "channel") => "channels".into(),
        ("parameterexecuteDAT", "pars") => "parameters".into(),
        ("baseCOMP", "Displaymode") => "displaymode".into(),
        ("feedbackTOP", "top") => "target".into(),
        ("feedbackCHOP", "chop") => "target".into(),
        ("noiseTOP", "harmon") => "harmonics".into(),
        ("noiseTOP", "exp") => "exponent".into(),
        ("reorderTOP", "outputredchan") => "red".into(),
        ("reorderTOP", "outputgreenchan") => "green".into(),
        ("reorderTOP", "outputbluechan") => "blue".into(),
        ("reorderTOP", "outputalphachan") => "alpha".into(),
        ("glslTOP", k) if k.starts_with("const") || k.starts_with("vec") => k.to_string(),
        (_, "inputfiltertype") => "filtertype".into(),
        (_, "chops") => "chop".into(),
        (_, "const0name") => "name0".into(),
        (_, "const0value") => "value0".into(),
        (_, "amp") => "amplitude".into(),
        (_, "wavetype") => "type".into(),
        (_, "renameto") => "to".into(),
        (_, "renamefrom") => "from".into(),
        ("constantCHOP", k) if k.starts_with("const") && k.ends_with("name") => {
            k.replacen("const", "name", 1)
        }
        ("constantCHOP", k) if k.starts_with("const") && k.ends_with("value") => {
            k.replacen("const", "value", 1)
        }
        _ => td_key.to_string(),
    }
}

pub(crate) fn map_td_value(otd_op: &str, td_key: &str, value: Value) -> Value {
    if otd_op == "lfoCHOP" && td_key == "wavetype" {
        if let Value::Int(i) = value {
            let items = ["sine", "triangle", "ramp", "square", "pulse"];
            return Value::Select(items.get(i as usize).unwrap_or(&"sine").to_string());
        }
    }
    if otd_op == "reorderTOP" && td_key.starts_with("output") {
        if let Value::Str(s) = &value {
            return Value::Str(map_reorder_channel(s));
        }
    }
    value
}

fn map_reorder_channel(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "r" | "red" | "input1r" | "input1.r" => "input1 r".into(),
        "g" | "green" | "input1g" | "input1.g" => "input1 g".into(),
        "b" | "blue" | "input1b" | "input1.b" => "input1 b".into(),
        "a" | "alpha" | "input1a" | "input1.a" => "input1 a".into(),
        "zero" | "0" => "zero".into(),
        "one" | "1" => "one".into(),
        "lum" | "luma" | "luminance" | "input1lum" | "input1.luma" => "input1 luma".into(),
        "input2r" | "input2.r" => "input2 r".into(),
        "input2g" | "input2.g" => "input2 g".into(),
        "input2b" | "input2.b" => "input2 b".into(),
        "input2a" | "input2.a" => "input2 a".into(),
        "input2lum" | "input2.luma" | "input2luma" => "input2 luma".into(),
        _ => s.to_string(),
    }
}

pub(crate) fn coerce_for_param(param: &Param, value: Value) -> Value {
    if param.menu.is_some() {
        return match value {
            Value::Bool(b) => Value::Select(if b { "on".into() } else { "off".into() }),
            Value::Str(s) | Value::Select(s) => Value::Select(s),
            Value::Int(i) => menu_at(param, i).unwrap_or(Value::Int(i)),
            Value::Float(f) => menu_at(param, f.round() as i64).unwrap_or(Value::Float(f)),
            other => other,
        };
    }
    match (&param.value, value) {
        (Value::Int(_), Value::Float(f)) => Value::Int(f.round() as i64),
        (Value::Float(_), Value::Int(i)) => Value::Float(i as f64),
        (Value::Int(_), Value::Str(s)) => s
            .trim()
            .parse::<f64>()
            .map(|f| Value::Int(f.round() as i64))
            .unwrap_or(Value::Str(s)),
        (Value::Float(_), Value::Str(s)) => s
            .trim()
            .parse::<f64>()
            .map(Value::Float)
            .unwrap_or(Value::Str(s)),
        (_, value) => value,
    }
}

pub(crate) fn menu_at(param: &Param, index: i64) -> Option<Value> {
    let menu = param.menu.as_ref()?;
    let item = menu.get(index.max(0) as usize)?;
    Some(Value::Select(item.value.clone()))
}

pub(crate) fn param_for_td_value(value: &Value) -> Param {
    match value {
        Value::Float(f) => Param::float(*f),
        Value::Int(i) => Param::int(*i),
        Value::Bool(b) => Param::bool(*b),
        Value::Str(s) => Param::str(s),
        Value::Select(s) => Param::new(Value::Select(s.clone())),
        other => Param::new(other.clone()),
    }
}

pub(crate) fn td_mode_uses_expression(mode: u32) -> bool {
    matches!(mode, 17 | 49)
}