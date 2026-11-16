use super::*;

/// `ch('/lfo1', 'chan1')` — a CHOP channel's current value.
#[pyfunction]
#[pyo3(signature = (op_path, channel = "chan1"))]
pub(crate) fn ch(op_path: &str, channel: &str) -> f64 {
    with_network(|net, _| net.channel(op_path, channel).unwrap_or(0.0) as f64).unwrap_or(0.0)
}

/// `setpar('/blur1', 'size', 12)` — ask for a parameter change.
///
/// Queued, not applied: the cook is in progress and must see one unchanging
/// graph. The host applies the queue between frames, so the change lands next
/// frame. Nothing here can fail loudly — a bad path is reported when the queue
/// is applied, because that is where it is discovered.
#[pyfunction]
pub(crate) fn setpar(op_path: &str, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
    let value = py_to_value(value).map_err(pyo3::exceptions::PyTypeError::new_err)?;
    let path = with_network(|net, node_path| {
        net.resolve_operator_path(node_path, op_path)
            .unwrap_or_else(|| resolve_py_op_path(op_path, node_path))
    })
    .unwrap_or_else(|| resolve_py_op_path(op_path, ""));
    script_actions().queue_param(&path, name, value);
    Ok(())
}
