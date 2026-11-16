pub(crate) fn resolve_py_op_path(op_path: &str, node_path: &str) -> String {
    let op_path = op_path.trim();
    if op_path.starts_with('/') {
        return op_path.to_string();
    }
    let base = node_path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
    if base.is_empty() {
        format!("/{op_path}")
    } else {
        format!("{base}/{op_path}")
    }
}
