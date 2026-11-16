use otd_core::value::Value;

#[derive(Debug, Clone)]
pub(crate) struct ParsedParm {
    pub(crate) key: String,
    pub(crate) td_mode: u32,
    pub(crate) constant: Option<Value>,
    pub(crate) expression: Option<String>,
}

pub fn parse_parm_block(text: &str) -> Vec<ParsedParm> {
    let mut out = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        let line = line.trim();
        if line == "?" {
            in_block = !in_block;
            continue;
        }
        if !in_block || line.is_empty() {
            continue;
        }
        if let Some(p) = parse_parm_line(line) {
            out.push(p);
        }
    }
    out
}

pub(crate) fn parse_parm_line(line: &str) -> Option<ParsedParm> {
    let mut parts = line.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 2 {
        return None;
    }
    let key = parts.remove(0).to_string();
    let mode: u32 = parts.first()?.parse().ok()?;
    parts.remove(0);
    if parts.is_empty() {
        return Some(ParsedParm {
            key,
            td_mode: mode,
            constant: None,
            expression: None,
        });
    }

    match mode {
        0 => {
            let s = parts.join(" ");
            Some(ParsedParm {
                key,
                td_mode: mode,
                constant: Some(value_from_td_string(&s)),
                expression: None,
            })
        }
        16 => {
            let default = parts.first().copied().unwrap_or("0");
            let expr = parts[1..].join(" ");
            Some(ParsedParm {
                key,
                td_mode: mode,
                constant: Some(value_from_td_string(default)),
                expression: if expr.is_empty() { None } else { Some(expr) },
            })
        }
        32 => {
            let s = parts.join(" ");
            Some(ParsedParm {
                key,
                td_mode: mode,
                constant: Some(Value::Str(s)),
                expression: None,
            })
        }
        17 | 49 => {
            let expr = parts.last().map(|s| s.to_string()).unwrap_or_default();
            let default = parts.first().copied().unwrap_or("");
            Some(ParsedParm {
                key,
                td_mode: mode,
                constant: Some(value_from_td_string(default)),
                expression: if expr.is_empty() { None } else { Some(expr) },
            })
        }
        2 => {
            let s = parts.join(" ");
            Some(ParsedParm {
                key,
                td_mode: mode,
                constant: Some(value_from_td_string(&s)),
                expression: None,
            })
        }
        _ => {
            let s = parts.join(" ");
            Some(ParsedParm {
                key,
                td_mode: mode,
                constant: Some(value_from_td_string(&s)),
                expression: None,
            })
        }
    }
}

fn unquote_td(s: &str) -> &str {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        &t[1..t.len() - 1]
    } else {
        s
    }
}

fn value_from_td_string(s: &str) -> Value {
    let s = unquote_td(s);
    if let Ok(f) = s.parse::<f64>() {
        return Value::Float(f);
    }
    if let Ok(i) = s.parse::<i64>() {
        return Value::Int(i);
    }
    match s {
        "on" | "On" | "ON" => Value::Bool(true),
        "off" | "Off" | "OFF" => Value::Bool(false),
        _ => Value::Str(s.to_string()),
    }
}