pub(crate) fn ns_error<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}