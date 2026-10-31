use std::ffi::CString;

use pyo3::exceptions::PySyntaxError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LintSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PythonLint {
    pub line: u32,
    pub column: u32,
    pub span_len: u32,
    pub message: String,
    pub severity: LintSeverity,
}

fn syntax_from_err(py: Python<'_>, err: &PyErr) -> PythonLint {
    let mut line = 1u32;
    let mut column = 0u32;
    let mut message = crate::format_error(py, err);
    if err.is_instance_of::<PySyntaxError>(py) {
        let value = err.value(py);
        if let Ok(l) = value.getattr("lineno").and_then(|v| v.extract::<Option<u32>>()) {
            if let Some(l) = l {
                line = l.max(1);
            }
        }
        if let Ok(c) = value.getattr("offset").and_then(|v| v.extract::<Option<u32>>()) {
            column = c.unwrap_or(0);
        }
        if let Ok(msg) = value.getattr("msg").and_then(|v| v.extract::<String>()) {
            message = msg;
        }
    }
    PythonLint {
        line,
        column,
        span_len: 0,
        message,
        severity: LintSeverity::Error,
    }
}

const LINT_FN: &str = r#"
def _otd_py_lint(source, filename):
    import ast
    out = []
    try:
        compile(source, filename, "exec")
    except SyntaxError as e:
        out.append((e.lineno or 1, e.offset or 0, str(e.msg or e), "error"))
        return out
    try:
        tree = ast.parse(source, filename)
    except SyntaxError as e:
        out.append((e.lineno or 1, e.offset or 0, str(e.msg or e), "error"))
        return out
    for node in ast.walk(tree):
        if not isinstance(node, ast.FunctionDef):
            continue
        params = {a.arg for a in node.args.args}
        params.update(a.arg for a in node.args.kwonlyargs)
        if node.args.vararg:
            params.add(node.args.vararg.arg)
        if node.args.kwarg:
            params.add(node.args.kwarg.arg)
        assigned = {}
        loaded = set()
        for sub in ast.walk(node):
            if isinstance(sub, ast.Name):
                if isinstance(sub.ctx, ast.Store):
                    assigned[sub.id] = sub.lineno
                elif isinstance(sub.ctx, ast.Load):
                    loaded.add(sub.id)
        for name, lineno in assigned.items():
            if name in loaded or name in params or name.startswith("_"):
                continue
            out.append((lineno, 0, f"local variable {name!r} is assigned but never used", "warning"))
    return out
"#;

pub fn lint_script(
    py: Python<'_>,
    startup_error: Option<&str>,
    source: &str,
    filename: &str,
) -> Vec<PythonLint> {
    if let Some(e) = startup_error {
        return vec![PythonLint {
            line: 1,
            column: 0,
            span_len: 0,
            message: e.to_string(),
            severity: LintSeverity::Error,
        }];
    }
    if source.is_empty() {
        return Vec::new();
    }
    let Ok(src) = CString::new(source) else {
        return vec![PythonLint {
            line: 1,
            column: 0,
            span_len: 0,
            message: "script contains a NUL byte".into(),
            severity: LintSeverity::Error,
        }];
    };

    let Ok(builtins) = py.import("builtins") else {
        return Vec::new();
    };
    if let Err(err) = builtins.call_method1("compile", (src.to_str().unwrap(), filename, "exec")) {
        return vec![syntax_from_err(py, &err)];
    }

    let locals = PyDict::new(py);
    let lint_src = CString::new(LINT_FN).expect("lint source");
    if py.run(&lint_src, None, Some(&locals)).is_err() {
        return Vec::new();
    }
    let Some(func) = locals.get_item("_otd_py_lint").ok().flatten() else {
        return Vec::new();
    };
    let Ok(result) = func.call1((source, filename)) else {
        return Vec::new();
    };
    let Ok(list) = result.cast::<PyList>() else {
        return Vec::new();
    };
    let mut issues = Vec::new();
    for item in list.iter() {
        let Ok(row) = item.cast::<PyTuple>() else {
            continue;
        };
        if row.len() != 4 {
            continue;
        }
        let line: u32 = row.get_item(0).ok().and_then(|v| v.extract().ok()).unwrap_or(1);
        let column: u32 = row.get_item(1).ok().and_then(|v| v.extract().ok()).unwrap_or(0);
        let message: String = row
            .get_item(2)
            .ok()
            .and_then(|v| v.extract().ok())
            .unwrap_or_default();
        let kind: String = row
            .get_item(3)
            .ok()
            .and_then(|v| v.extract().ok())
            .unwrap_or_else(|| "warning".into());
        issues.push(PythonLint {
            line: line.max(1),
            column,
            span_len: 0,
            message,
            severity: if kind == "error" {
                LintSeverity::Error
            } else {
                LintSeverity::Warning
            },
        });
    }
    issues
}
