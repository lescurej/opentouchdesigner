use std::collections::BTreeMap;

use otd_core::Value;

pub(crate) const DELETE_KEYS: &[&str] = &["delete", "deletes", "remove", "removals"];
pub(crate) const SET_KEYS: &[&str] = &["set", "sets"];
const OTHER_KEYS: &[&str] = &["notes", "nodes", "connections", "viewer"];

pub(crate) fn first_array<'a>(
    value: &'a serde_json::Value,
    keys: &[&str],
) -> Option<&'a Vec<serde_json::Value>> {
    keys.iter().find_map(|k| value.get(*k)?.as_array())
}

pub(crate) fn unknown_keys(value: &serde_json::Value) -> Vec<String> {
    let Some(object) = value.as_object() else {
        return Vec::new();
    };
    object
        .iter()
        .filter(|(_, v)| !v.is_null())
        .filter(|(_, v)| !matches!(v, serde_json::Value::Array(a) if a.is_empty()))
        .map(|(k, _)| k)
        .filter(|k| {
            !OTHER_KEYS.contains(&k.as_str())
                && !SET_KEYS.contains(&k.as_str())
                && !DELETE_KEYS.contains(&k.as_str())
        })
        .cloned()
        .collect()
}

pub(crate) fn string_map(value: Option<&serde_json::Value>) -> BTreeMap<String, String> {
    value
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn split_export(target: &str) -> Option<(&str, &str)> {
    let (node, channel) = target.trim().rsplit_once(':')?;
    let (node, channel) = (node.trim(), channel.trim());
    (!node.is_empty() && !channel.is_empty()).then_some((node, channel))
}

pub(crate) fn json_to_value(raw: &serde_json::Value, declared: &Value) -> Option<Value> {
    if let Some(items) = raw.as_array() {
        let n: Vec<f64> = items.iter().filter_map(|v| v.as_f64()).collect();
        return match (declared, n.len()) {
            (Value::Vec2(_), 2..) => Some(Value::Vec2([n[0], n[1]])),
            (Value::Vec3(_), 3..) => Some(Value::Vec3([n[0], n[1], n[2]])),
            (Value::Vec4(_), 4..) => Some(Value::Vec4([n[0], n[1], n[2], n[3]])),
            (Value::Vec4(_), 3) => Some(Value::Vec4([n[0], n[1], n[2], 1.0])),
            _ => None,
        };
    }
    Some(match declared {
        Value::Str(_) => Value::Str(match raw.as_str() {
            Some(s) => s.to_string(),
            None => raw.to_string(),
        }),
        Value::Bool(_) => Value::Bool(raw.as_bool().or_else(|| raw.as_f64().map(|f| f != 0.0))?),
        Value::Int(_) => Value::Int(raw.as_i64().or_else(|| raw.as_f64().map(|f| f as i64))?),
        Value::Float(_) => Value::Float(raw.as_f64()?),
        Value::Vec2(_) | Value::Vec3(_) | Value::Vec4(_) => {
            let f = raw.as_f64()?;
            match declared {
                Value::Vec2(_) => Value::Vec2([f, f]),
                Value::Vec3(_) => Value::Vec3([f, f, f]),
                _ => Value::Vec4([f, f, f, 1.0]),
            }
        }
        Value::Select(_) => Value::Select(match raw.as_str() {
            Some(s) => s.to_string(),
            None => raw.to_string(),
        }),
    })
}