//! Patch planning: catalogue, prompts, parse, apply.

use std::collections::BTreeMap;

use otd_core::{OpRegistry, Value};


/// Every operator, compactly, for the model to read.
pub fn catalogue(registry: &OpRegistry) -> String {
    let mut by_family: BTreeMap<&'static str, Vec<&otd_core::OpDef>> = BTreeMap::new();
    for def in registry.iter() {
        by_family.entry(def.family.suffix()).or_default().push(def);
    }
    let mut out = String::new();
    for (family, mut defs) in by_family {
        defs.sort_by_key(|d| d.type_name);
        out.push_str(&format!("\n## {family}\n"));
        for def in defs {
            let inputs = if def.inputs.is_empty() {
                "none".to_string()
            } else {
                def.inputs.join(",")
            };
            out.push_str(&format!("{} in[{}]", def.type_name, inputs));
            let params = (def.params)();
            for (key, p) in &params {
                out.push(' ');
                out.push_str(key);
                if let Some(menu) = &p.menu {
                    let body = menu
                        .iter()
                        .map(|item| {
                            if item.option == item.value || item.value.is_empty() {
                                item.option.clone()
                            } else {
                                format!("{}={}", item.option, item.value)
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("|");
                    out.push_str(&format!("={{{body}}}"));
                } else {
                    out.push_str(&format!("={}", short_value(&p.value)));
                }
            }
            // Last, so the parameter list stays a single unbroken run of
            // `key=value` and the prose cannot be mistaken for one.
            out.push_str(&format!(" — {}", def.summary));
            out.push('\n');
        }
    }
    out
}

pub(crate) fn short_value(v: &Value) -> String {
    match v {
        Value::Str(s) if s.len() > 12 => "str".into(),
        Value::Str(s) if s.is_empty() => "str".into(),
        Value::Str(s) => s.clone(),
        Value::Select(s) if s.is_empty() => "select".into(),
        Value::Select(s) => s.clone(),
        Value::Float(f) => format!("{f}"),
        Value::Int(i) => format!("{i}"),
        Value::Bool(b) => format!("{b}"),
        Value::Vec2(v) => format!("[{},{}]", v[0], v[1]),
        Value::Vec3(v) => format!("[{},{},{}]", v[0], v[1], v[2]),
        Value::Vec4(v) => format!("[{},{},{},{}]", v[0], v[1], v[2], v[3]),
    }
}
