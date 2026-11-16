use super::*;

#[pyfunction]
pub(crate) fn op(op_path: &str) -> OpRef {
    OpRef {
        path: op_path.to_string(),
    }
}

/// `par('/blur1', 'size')` — another operator's parameter value.
#[pyfunction]
pub(crate) fn par(py: Python<'_>, op_path: &str, name: &str) -> Py<PyAny> {
    let path = with_network(|net, node_path| {
        net.resolve_operator_path(node_path, op_path)
            .unwrap_or_else(|| resolve_py_op_path(op_path, node_path))
    })
    .unwrap_or_else(|| resolve_py_op_path(op_path, ""));
    with_network(|net, _| net.param_value(&path, name))
        .flatten()
        .map(|v| value_to_py(py, &v))
        .unwrap_or_else(|| py.None())
}

#[pyfunction]
pub(crate) fn mod_op(py: Python<'_>, op_path: &str) -> PyResult<Py<PyAny>> {
    let abs = with_network(|net, node_path| {
        net.resolve_operator_path(node_path, op_path)
            .unwrap_or_else(|| resolve_py_op_path(op_path, node_path))
    })
    .unwrap_or_else(|| resolve_py_op_path(op_path, ""));
    let source = with_network(|net, _| net.dat_text(&abs))
        .flatten()
        .unwrap_or_default();
    if source.trim().is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err("DAT text is empty"));
    }
    let types = py.import("types")?;
    let module = types.getattr("ModuleType")?.call1((format!("mod_{abs}"),))?;
    let module_dict = module
        .cast::<PyDict>()
        .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("module type"))?;
    py.run(
        &CString::new(source).map_err(|_| {
            pyo3::exceptions::PyValueError::new_err("DAT source contains a NUL byte")
        })?,
        None,
        Some(module_dict),
    )
    .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format_error(py, &e)))?;
    Ok(module.unbind())
}

/// `parent('speed')` — a custom parameter on the enclosing component.
#[pyfunction]
pub(crate) fn parent(py: Python<'_>, name: &str) -> Py<PyAny> {
    with_network(|net, path| net.parent_param(path, name))
        .flatten()
        .map(|v| value_to_py(py, &v))
        .unwrap_or_else(|| py.None())
}

pub(crate) fn value_to_py(py: Python<'_>, v: &Value) -> Py<PyAny> {
    match v {
        Value::Float(f) => f.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Int(i) => i.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        Value::Str(s) | Value::Select(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Vec2(a) => PyTuple::new(py, a).unwrap().into_any().unbind(),
        Value::Vec3(a) => PyTuple::new(py, a).unwrap().into_any().unbind(),
        Value::Vec4(a) => PyTuple::new(py, a).unwrap().into_any().unbind(),
    }
}

/// Convert a Python result back to a parameter value.
pub(crate) fn py_to_value(obj: &Bound<'_, PyAny>) -> Result<Value, String> {
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(f) = obj.extract::<f64>() {
        return Ok(Value::Float(f));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Int(i));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::Str(s));
    }
    if let Ok(seq) = obj.extract::<Vec<f64>>() {
        return match seq.len() {
            2 => Ok(Value::Vec2([seq[0], seq[1]])),
            3 => Ok(Value::Vec3([seq[0], seq[1], seq[2]])),
            4 => Ok(Value::Vec4([seq[0], seq[1], seq[2], seq[3]])),
            _ => Err(format!(
                "a sequence of {} is not a parameter value",
                seq.len()
            )),
        };
    }
    Err(format!(
        "expression produced {}, which is not a parameter value",
        obj.get_type()
            .name()
            .map(|n| n.to_string())
            .unwrap_or_default()
    ))
}
