use super::*;

mod core;
mod eval;
mod callbacks;
mod webserver;
mod edits;
mod default;

pub struct PyEngine {
    globals: Py<PyDict>,
    cache: HashMap<String, Py<PyAny>>,
    namespaces: HashMap<String, Py<PyDict>>,
    pub startup_error: Option<String>,
    script_log: Vec<String>,
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
