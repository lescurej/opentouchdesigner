use super::*;

pub(crate) fn format_console_value(v: &Value) -> String {
    match v {
        Value::Float(f) => format!("{f}"),
        Value::Int(i) => format!("{i}"),
        Value::Bool(b) => format!("{b}"),
        Value::Str(s) | Value::Select(s) => s.clone(),
        Value::Vec2(a) => format!("({:.4}, {:.4})", a[0], a[1]),
        Value::Vec3(a) => format!("({:.4}, {:.4}, {:.4})", a[0], a[1], a[2]),
        Value::Vec4(a) => format!(
            "({:.4}, {:.4}, {:.4}, {:.4})",
            a[0], a[1], a[2], a[3]
        ),
    }
}

pub(crate) fn merge_console_out(captured: &str, v: &Value) -> String {
    let formatted = format_console_value(v);
    if captured.is_empty() {
        formatted
    } else if formatted.is_empty() {
        captured.to_string()
    } else {
        format!("{captured}\n{formatted}")
    }
}

pub(crate) fn drain_captured_stdout(_py: Python<'_>, cap: &Bound<'_, PyAny>) -> String {
    cap.call_method0("getvalue")
        .and_then(|v| v.extract())
        .unwrap_or_default()
}

pub(crate) fn restore_stdio(py: Python<'_>, sys: &Bound<'_, PyModule>, out: Py<PyAny>, err: Py<PyAny>) {
    let _ = sys.setattr("stdout", out.bind(py));
    let _ = sys.setattr("stderr", err.bind(py));
}

type StdioCapture<'py> = (
    Bound<'py, pyo3::types::PyModule>,
    Py<PyAny>,
    Py<PyAny>,
    Bound<'py, PyAny>,
);

pub(crate) fn capture_stdio(py: Python<'_>) -> Result<StdioCapture<'_>, String> {
    let sys = py
        .import("sys")
        .map_err(|e| format_error(py, &e))?;
    let io = py.import("io").map_err(|e| format_error(py, &e))?;
    let cap = io
        .getattr("StringIO")
        .map_err(|e| format_error(py, &e))?
        .call0()
        .map_err(|e| format_error(py, &e))?;
    let old_out = sys.getattr("stdout").map_err(|e| format_error(py, &e))?.unbind();
    let old_err = sys.getattr("stderr").map_err(|e| format_error(py, &e))?.unbind();
    sys.setattr("stdout", &cap)
        .map_err(|e| format_error(py, &e))?;
    sys.setattr("stderr", &cap)
        .map_err(|e| format_error(py, &e))?;
    Ok((sys, old_out, old_err, cap))
}

/// Turn a Python exception into the one line the parameter panel has room for.
pub(crate) fn format_error(py: Python<'_>, err: &PyErr) -> String {
    let kind = err
        .get_type(py)
        .name()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "Error".into());
    let msg = err.value(py).to_string();
    if msg.is_empty() {
        kind
    } else {
        format!("{kind}: {msg}")
    }
}
