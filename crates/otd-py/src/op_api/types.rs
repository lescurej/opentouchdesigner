use super::*;


#[pyclass(skip_from_py_object)]
#[derive(Clone)]
pub(crate) struct OpRef {
    pub(crate) path: String,
}

pub(crate) fn resolve_op_ref_path(op_ref: &str) -> String {
    let op_ref = op_ref.trim();
    if op_ref.is_empty() || op_ref == "." {
        return with_network(|_, path| path.to_string()).unwrap_or_default();
    }
    with_network(|net, node_path| {
        net.resolve_operator_path(node_path, op_ref)
            .unwrap_or_else(|| resolve_py_op_path(op_ref, node_path))
    })
    .unwrap_or_else(|| resolve_py_op_path(op_ref, ""))
}

pub(crate) fn me_op_ref() -> OpRef {
    OpRef {
        path: String::new(),
    }
}

#[pyclass]
pub(crate) struct ParVal {
    pub(crate) op_path: String,
    pub(crate) name: String,
}

#[pymethods]
impl ParVal {
    #[getter]
    fn val(&self, py: Python<'_>) -> Py<PyAny> {
        let path = resolve_op_ref_path(&self.op_path);
        with_network(|net, _| net.param_value(&path, &self.name))
            .flatten()
            .map(|v| value_to_py(py, &v))
            .unwrap_or_else(|| py.None())
    }
}

pub(crate) fn client_id(client: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = client.extract::<String>() {
        return Ok(s);
    }
    client
        .str()
        .map(|s| s.to_string())
        .map_err(|_| pyo3::exceptions::PyTypeError::new_err("client must be a string"))
}

pub(crate) fn py_osc_args(args: Bound<'_, PyList>) -> PyResult<Vec<OscArg>> {
    let mut out = Vec::new();
    for item in args.iter() {
        if let Ok(i) = item.extract::<i64>() {
            out.push(OscArg::Int(i as i32));
        } else if let Ok(f) = item.extract::<f64>() {
            out.push(OscArg::Float(f as f32));
        } else if let Ok(s) = item.extract::<String>() {
            out.push(OscArg::Str(s));
        } else if let Ok(b) = item.extract::<Vec<u8>>() {
            out.push(OscArg::Blob(b));
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "OSC args must be int, float, str, or bytes",
            ));
        }
    }
    Ok(out)
}
