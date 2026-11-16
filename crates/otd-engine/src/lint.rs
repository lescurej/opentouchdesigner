use otd_core::Graph;
use otd_py::{LintSeverity, PythonLint};

pub fn lint_op_paths(graph: &Graph, me_path: &str, source: &str) -> Vec<PythonLint> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < source.len() {
        if source.get(i..).is_some_and(|s| s.starts_with("op(")) {
            let mut j = i + 3;
            if let Some(lit) = read_string_literal(source, &mut j) {
                let resolved = resolve_op_ref(me_path, lit.text);
                if graph.find(&resolved).is_none() {
                    out.push(PythonLint {
                        line: lit.line,
                        column: lit.column,
                        span_len: lit.text.chars().count() as u32,
                        message: format!("unknown operator {:?} (resolved {resolved})", lit.text),
                        severity: LintSeverity::Warning,
                    });
                }
            }
            i = j;
        } else {
            i = next_char_boundary(source, i + 1);
        }
    }
    out
}

pub fn lint_ch_paths(
    graph: &Graph,
    me_path: &str,
    source: &str,
    channel_names: impl Fn(&str) -> Vec<String>,
) -> Vec<PythonLint> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < source.len() {
        if source.get(i..).is_some_and(|s| s.starts_with("ch(")) {
            let mut j = i + 3;
            if let Some(op_lit) = read_string_literal(source, &mut j) {
                let resolved = resolve_op_ref(me_path, op_lit.text);
                if graph.find(&resolved).is_none() {
                    out.push(PythonLint {
                        line: op_lit.line,
                        column: op_lit.column,
                        span_len: op_lit.text.chars().count() as u32,
                        message: format!(
                            "unknown operator {:?} in ch() (resolved {resolved})",
                            op_lit.text
                        ),
                        severity: LintSeverity::Warning,
                    });
                }
                skip_ws(source, &mut j);
                if source.as_bytes().get(j) == Some(&b',') {
                    j += 1;
                    if let Some(chan_lit) = read_string_literal(source, &mut j) {
                        let names = channel_names(&resolved);
                        if !names.is_empty() && !names.iter().any(|n| n.as_str() == chan_lit.text) {
                            out.push(PythonLint {
                                line: chan_lit.line,
                                column: chan_lit.column,
                                span_len: chan_lit.text.chars().count() as u32,
                                message: format!(
                                    "unknown channel {:?} on {resolved}",
                                    chan_lit.text
                                ),
                                severity: LintSeverity::Warning,
                            });
                        }
                    }
                }
            }
            i = j;
        } else {
            i = next_char_boundary(source, i + 1);
        }
    }
    out
}

struct StringLit<'a> {
    text: &'a str,
    line: u32,
    column: u32,
}

fn read_string_literal<'a>(source: &'a str, j: &mut usize) -> Option<StringLit<'a>> {
    skip_ws(source, j);
    let quote = *source.as_bytes().get(*j)?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    *j += 1;
    let content_start = *j;
    while *j < source.len() {
        if !source.is_char_boundary(*j) {
            *j = next_char_boundary(source, *j);
            if *j >= source.len() {
                return None;
            }
        }
        let b = source.as_bytes()[*j];
        if b == quote {
            let content = source.get(content_start..*j)?;
            let prefix = source.get(..content_start).unwrap_or("");
            let line = prefix.matches('\n').count() as u32 + 1;
            let line_start = prefix.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let column = (content_start - line_start) as u32;
            *j += 1;
            return Some(StringLit {
                text: content,
                line,
                column,
            });
        }
        if b == b'\\' {
            *j += 1;
            if *j < source.len() {
                *j = next_char_boundary(source, *j + 1);
            }
        } else if b < 128 {
            *j += 1;
        } else {
            *j = next_char_boundary(source, *j + 1);
        }
    }
    None
}

fn next_char_boundary(source: &str, i: usize) -> usize {
    let i = i.min(source.len());
    if i >= source.len() || source.is_char_boundary(i) {
        return i;
    }
    (i..source.len())
        .find(|&b| source.is_char_boundary(b))
        .unwrap_or(source.len())
}

fn skip_ws(source: &str, j: &mut usize) {
    while *j < source.len() && source.as_bytes()[*j].is_ascii_whitespace() {
        *j += 1;
    }
}

pub fn resolve_op_ref(me_path: &str, op_path: &str) -> String {
    let op_path = op_path.trim();
    if op_path.starts_with('/') {
        return op_path.to_string();
    }
    let base = me_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    if base.is_empty() {
        format!("/{op_path}")
    } else {
        format!("{base}/{op_path}")
    }
}

#[cfg(test)]
#[path = "tests/lint__tests.rs"]
mod tests;
