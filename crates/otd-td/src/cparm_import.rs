//! TouchDesigner `.cparm` → component custom parameters.

use otd_core::graph::{Graph, NodeId};
use otd_core::param::{Param, SelectOption};
use otd_core::value::Value;

/// Names of author parameters declared in a `.cparm` file (the label TD stores).
pub fn custom_param_names(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| parse_cparm_line(line.trim()).map(|(key, _)| key))
        .collect()
}

/// Parse a component `.cparm` file and attach custom parameters to `comp`.
pub fn apply_cparm_block(graph: &mut Graph, comp: NodeId, text: &str) {
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line == "?" || line.starts_with("pages") {
            continue;
        }
        if let Some((key, param)) = parse_cparm_line(line) {
            graph.add_custom_param(comp, &key, param);
        }
    }
}

fn tokenize_cparm_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    let bytes = line.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            out.push(line[start..i].to_string());
            i += 1;
        } else if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else {
            let start = i;
            while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            out.push(line[start..i].to_string());
        }
    }
    out
}

fn empty_string_pair(toks: &[String]) -> Option<usize> {
    for (i, t) in toks.iter().enumerate() {
        if t.is_empty() && i + 1 < toks.len() && toks[i + 1].is_empty() {
            return Some(i);
        }
    }
    None
}

fn parse_cparm_line(line: &str) -> Option<(String, Param)> {
    let toks = tokenize_cparm_line(line);
    if toks.len() < 6 {
        return None;
    }
    toks[0].parse::<u64>().ok()?;
    let name = toks[1].clone();
    let label = if toks[2].is_empty() {
        name.clone()
    } else {
        toks[2].clone()
    };

    if let Some(mark) = toks.iter().position(|t| t == "4097") {
        let count: usize = toks.get(mark + 1)?.parse().ok()?;
        let mut items = Vec::new();
        let mut j = mark + 2;
        for _ in 0..count {
            let value = toks.get(j)?.clone();
            let opt = toks.get(j + 1).cloned().unwrap_or_else(|| value.clone());
            items.push(SelectOption::new(&opt, &value));
            j += 2;
        }
        let default = toks
            .get(empty_string_pair(&toks)? - 1)
            .cloned()
            .unwrap_or_else(|| items.first().map(|o| o.value.clone()).unwrap_or_default());
        let pick = items
            .iter()
            .find(|o| o.value == default)
            .or(items.first())?;
        let refs: Vec<&str> = items.iter().map(|o| o.value.as_str()).collect();
        let mut param = Param::menu(&pick.value, &refs);
        param.menu = Some(items);
        param.value = Value::Select(default);
        return Some((name, param.with_label(&label)));
    }

    let empty = empty_string_pair(&toks)?;
    let default_tok = toks.get(empty.checked_sub(1)?)?;
    let default_f = default_tok.parse::<f64>().ok();

    let lower = name.to_ascii_lowercase();
    let param = if lower.contains("pause")
        || lower.starts_with("run")
        || lower.ends_with("active")
        || (default_tok == "0" || default_tok == "1")
            && (lower.contains("sim") || lower.contains("enable") || lower.contains("mode"))
            && !default_tok.contains('.')
    {
        Param::bool(default_tok == "1").with_label(&label)
    } else if lower.contains("reset") || lower.ends_with("pulse") {
        Param::bool(false).with_label(&label)
    } else if default_tok.contains('.') {
        let mut p = Param::float(default_f.unwrap_or(0.0)).with_label(&label);
        if let (Some(min), Some(max)) = (
            toks.get(6).and_then(|s| s.parse::<f64>().ok()),
            toks.get(8).and_then(|s| s.parse::<f64>().ok()),
        ) {
            if min < max {
                p = p.with_range(min, max);
            }
        }
        p
    } else {
        Param::int(default_f.unwrap_or(0.0).round() as i64).with_label(&label)
    };

    Some((name, param))
}

#[cfg(test)]
#[path = "tests/cparm_import__tests.rs"]
mod tests;
