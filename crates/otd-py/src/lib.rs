//! `otd-py` — embedded CPython for parameter expressions.
//!
//! PLAN.md §3 calls Python "non-negotiable for TD migration", and §6 flags the
//! GIL against realtime: "scripts run at a fixed phase of the frame, never
//! block render". Both shape what this crate is.
//!
//! **Expressions are pure and short.** A parameter expression may read the
//! network but never mutate it — [`Network`] is the only thing reachable from
//! Python here, and it is read-only. That keeps evaluation reentrant with the
//! cook and means a runaway expression can only be slow, not corrupting.
//!
//! **Compiled code is cached.** Compiling on every frame for every parameter
//! would dominate the frame; sources are compiled once and keyed by content,
//! so editing an expression recompiles exactly one thing.
//!
//! **Failure is never fatal.** A broken expression reports its error and the
//! parameter falls back to its constant, because a typo during a show must not
//! stop the render.

use std::collections::HashMap;
use std::ffi::CString;

use otd_core::{
    callback::{CallbackGuard, with_network},
    EvalContext, OscArg, ParamEdit, Value, encode_message,
    flush_pending_sync_recooks, script_actions, take_queued_deferred_recooks, take_queued_edits,
};
use otd_dat::DatData;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};

mod lint;
pub use lint::{LintSeverity, PythonLint};

/// `ch('/lfo1', 'chan1')` — a CHOP channel's current value.
#[pyfunction]
#[pyo3(signature = (op_path, channel = "chan1"))]
fn ch(op_path: &str, channel: &str) -> f64 {
    with_network(|net, _| net.channel(op_path, channel).unwrap_or(0.0) as f64).unwrap_or(0.0)
}

/// `setpar('/blur1', 'size', 12)` — ask for a parameter change.
///
/// Queued, not applied: the cook is in progress and must see one unchanging
/// graph. The host applies the queue between frames, so the change lands next
/// frame. Nothing here can fail loudly — a bad path is reported when the queue
/// is applied, because that is where it is discovered.
#[pyfunction]
fn setpar(op_path: &str, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
    let value = py_to_value(value).map_err(pyo3::exceptions::PyTypeError::new_err)?;
    let path = with_network(|net, node_path| {
        net.resolve_operator_path(node_path, op_path)
            .unwrap_or_else(|| resolve_py_op_path(op_path, node_path))
    })
    .unwrap_or_else(|| resolve_py_op_path(op_path, ""));
    script_actions().queue_param(&path, name, value);
    Ok(())
}

fn after_script_invoke() -> Result<(), String> {
    flush_pending_sync_recooks()
}

fn queue_dat_text(path: &str, text: String) {
    let param = with_network(|net, _| net.dat_content_param(path))
        .flatten()
        .unwrap_or("text");
    script_actions().queue_dat_text(path, param, text);
}

fn queue_recook(path: &str, deferred: bool) {
    script_actions().run_op(path, deferred);
}

fn dat_grid_mut(path: &str, edit: impl FnOnce(&mut DatData) -> Result<(), String>) -> Result<(), String> {
    let mut data = DatData::table(dat_grid(path).unwrap_or_default());
    edit(&mut data)?;
    queue_dat_text(path, data.to_param_text());
    Ok(())
}

fn dat_grid(path: &str) -> Option<Vec<Vec<String>>> {
    with_network(|net, _| net.dat_rows(path)).flatten()
}

fn dat_num_rows(path: &str) -> i64 {
    dat_grid(path).map(|g| g.len() as i64).unwrap_or(0)
}

fn dat_num_cols(path: &str) -> i64 {
    dat_grid(path)
        .map(|g| g.iter().map(|r| r.len()).max().unwrap_or(0) as i64)
        .unwrap_or(0)
}

fn dat_get_cell(path: &str, row: i64, col: i64) -> String {
    if row < 0 || col < 0 {
        return String::new();
    }
    let row = row as usize;
    let col = col as usize;
    dat_grid(path)
        .and_then(|g| g.get(row).and_then(|r| r.get(col)).cloned())
        .unwrap_or_default()
}

fn dat_set_cell(path: &str, row: i64, col: i64, value: &str) -> Result<(), String> {
    if row < 0 || col < 0 {
        return Err("row and column must be >= 0".into());
    }
    dat_grid_mut(path, |data| {
        data.set_cell(row as usize, col as usize, value);
        Ok(())
    })
}

fn dat_clear(path: &str) {
    let _ = dat_grid_mut(path, |data| {
        data.clear_grid();
        Ok(())
    });
}

fn resolve_py_op_path(op_path: &str, node_path: &str) -> String {
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

#[pyclass(skip_from_py_object)]
#[derive(Clone)]
struct OpRef {
    path: String,
}

fn resolve_op_ref_path(op_ref: &str) -> String {
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

fn me_op_ref() -> OpRef {
    OpRef {
        path: String::new(),
    }
}

#[pyclass]
struct ParVal {
    op_path: String,
    name: String,
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

fn client_id(client: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = client.extract::<String>() {
        return Ok(s);
    }
    client
        .str()
        .map(|s| s.to_string())
        .map_err(|_| pyo3::exceptions::PyTypeError::new_err("client must be a string"))
}

fn py_osc_args(args: Bound<'_, PyList>) -> PyResult<Vec<OscArg>> {
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

#[pymethods]
#[allow(non_snake_case)]
impl OpRef {
    #[getter]
    fn name(&self) -> String {
        resolve_op_ref_path(&self.path)
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string()
    }

    #[getter]
    #[pyo3(name = "text")]
    fn get_text(&self) -> String {
        let abs = resolve_op_ref_path(&self.path);
        with_network(|net, _| net.dat_text(&abs))
            .flatten()
            .unwrap_or_default()
    }

    #[setter]
    #[pyo3(name = "text")]
    fn set_text(&self, value: &str) {
        let path = resolve_op_ref_path(&self.path);
        queue_dat_text(&path, value.to_string());
    }

    #[getter]
    fn numRows(&self) -> i64 {
        dat_num_rows(&resolve_op_ref_path(&self.path))
    }

    #[getter]
    fn numCols(&self) -> i64 {
        dat_num_cols(&resolve_op_ref_path(&self.path))
    }

    #[pyo3(name = "getCell")]
    fn get_cell(&self, row: i64, col: i64) -> String {
        dat_get_cell(&resolve_op_ref_path(&self.path), row, col)
    }

    #[pyo3(name = "setCell")]
    fn set_cell(&self, row: i64, col: i64, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = value.str()?.to_string();
        let path = resolve_op_ref_path(&self.path);
        dat_set_cell(&path, row, col, &value).map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    fn clear(&self) {
        dat_clear(&resolve_op_ref_path(&self.path));
    }

    #[pyo3(signature = (deferred=false))]
    fn run(&self, deferred: bool) {
        queue_recook(&resolve_op_ref_path(&self.path), deferred);
    }

    #[pyo3(name = "appendRow")]
    fn append_row(&self, cells: Bound<'_, PyList>) -> PyResult<()> {
        let path = resolve_op_ref_path(&self.path);
        let row: Result<Vec<String>, String> = cells
            .iter()
            .map(|c| c.str().map(|s| s.to_string()).map_err(|e| e.to_string()))
            .collect();
        let row = row.map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        dat_grid_mut(&path, |data| {
            data.append_row(&row.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            Ok(())
        })
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "appendCol")]
    fn append_col(&self, default: Option<&str>) -> PyResult<()> {
        let path = resolve_op_ref_path(&self.path);
        let default = default.unwrap_or("");
        dat_grid_mut(&path, |data| {
            data.append_col(default);
            Ok(())
        })
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "insertRow")]
    fn insert_row(&self, index: i64, cells: Bound<'_, PyList>) -> PyResult<()> {
        if index < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err("row index must be >= 0"));
        }
        let path = resolve_op_ref_path(&self.path);
        let row: Result<Vec<String>, String> = cells
            .iter()
            .map(|c| c.str().map(|s| s.to_string()).map_err(|e| e.to_string()))
            .collect();
        let row = row.map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        dat_grid_mut(&path, |data| {
            data.insert_row(index as usize, &row.iter().map(|s| s.as_str()).collect::<Vec<_>>());
            Ok(())
        })
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "deleteRow")]
    fn delete_row(&self, index: i64) -> PyResult<()> {
        if index < 0 {
            return Err(pyo3::exceptions::PyValueError::new_err("row index must be >= 0"));
        }
        let path = resolve_op_ref_path(&self.path);
        dat_grid_mut(&path, |data| {
            data.delete_row(index as usize);
            Ok(())
        })
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    fn send(&self, message: &str) -> PyResult<()> {
        let path = resolve_op_ref_path(&self.path);
        script_actions()
            .udp_send(&path, message.as_bytes())
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "sendOSC")]
    fn send_osc(&self, address: &str, args: Bound<'_, PyList>) -> PyResult<()> {
        let path = resolve_op_ref_path(&self.path);
        let args = py_osc_args(args)?;
        let payload = encode_message(address, &args)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        script_actions()
            .udp_send(&path, &payload)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "webSocketSendText")]
    fn web_socket_send_text(&self, client: &Bound<'_, PyAny>, data: &str) -> PyResult<()> {
        let server = resolve_op_ref_path(&self.path);
        let client = client_id(client)?;
        script_actions()
            .ws_send_text(&server, &client, data)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "webSocketSendBinary")]
    fn web_socket_send_binary(
        &self,
        client: &Bound<'_, PyAny>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let server = resolve_op_ref_path(&self.path);
        let client = client_id(client)?;
        let bytes = if let Ok(s) = data.extract::<Vec<u8>>() {
            s
        } else {
            data.extract::<String>()?.into_bytes()
        };
        script_actions()
            .ws_send_binary(&server, &client, &bytes)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    #[pyo3(name = "webSocketSendPong")]
    #[pyo3(signature = (client, data=None))]
    fn web_socket_send_pong(
        &self,
        client: &Bound<'_, PyAny>,
        data: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let server = resolve_op_ref_path(&self.path);
        let client = client_id(client)?;
        let bytes = match data {
            None => Vec::new(),
            Some(d) => d
                .extract::<Vec<u8>>()
                .or_else(|_| d.extract::<String>().map(|s| s.into_bytes()))
                .unwrap_or_default(),
        };
        script_actions()
            .ws_send_pong(&server, &client, &bytes)
            .map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    fn __setitem__(
        &self,
        key: Bound<'_, PyAny>,
        value: Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let (row, col) = key
            .extract::<(i64, i64)>()
            .map_err(|_| pyo3::exceptions::PyTypeError::new_err("DAT assignment uses [row, col]"))?;
        let value = value.str()?.to_string();
        let path = resolve_op_ref_path(&self.path);
        dat_set_cell(&path, row, col, &value).map_err(pyo3::exceptions::PyRuntimeError::new_err)
    }

    fn __getattr__(&self, py: Python<'_>, name: &str) -> PyResult<Py<PyAny>> {
        let par = ParVal {
            op_path: self.path.clone(),
            name: name.to_string(),
        };
        Ok(par.into_pyobject(py)?.into_any().unbind())
    }

    fn __getitem__(&self, py: Python<'_>, key: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        if let Ok(channel) = key.extract::<String>() {
            let path = resolve_op_ref_path(&self.path);
            let v = with_network(|net, _| net.channel(&path, &channel).unwrap_or(0.0) as f64)
                .unwrap_or(0.0);
            return Ok(v.into_pyobject(py)?.into_any().unbind());
        }
        if let Ok((row, col)) = key.extract::<(i64, i64)>() {
            let path = resolve_op_ref_path(&self.path);
            let s = dat_get_cell(&path, row, col);
            return Ok(s.into_pyobject(py)?.into_any().unbind());
        }
        Err(pyo3::exceptions::PyTypeError::new_err(
            "use op('x')['chan'] for CHOP channels or op('x')[row, col] for DAT cells",
        ))
    }

    fn par(&self, py: Python<'_>, name: &str) -> Py<PyAny> {
        ParVal {
            op_path: self.path.clone(),
            name: name.to_string(),
        }
        .into_pyobject(py)
        .map(|o| o.into_any().unbind())
        .unwrap_or_else(|_| py.None())
    }
}

#[pyfunction]
fn op(op_path: &str) -> OpRef {
    OpRef {
        path: op_path.to_string(),
    }
}

/// `par('/blur1', 'size')` — another operator's parameter value.
#[pyfunction]
fn par(py: Python<'_>, op_path: &str, name: &str) -> Py<PyAny> {
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
fn mod_op(py: Python<'_>, op_path: &str) -> PyResult<Py<PyAny>> {
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
fn parent(py: Python<'_>, name: &str) -> Py<PyAny> {
    with_network(|net, path| net.parent_param(path, name))
        .flatten()
        .map(|v| value_to_py(py, &v))
        .unwrap_or_else(|| py.None())
}

fn value_to_py(py: Python<'_>, v: &Value) -> Py<PyAny> {
    match v {
        Value::Float(f) => f.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Int(i) => i.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Bool(b) => b.into_pyobject(py).unwrap().to_owned().into_any().unbind(),
        Value::Str(s) => s.into_pyobject(py).unwrap().into_any().unbind(),
        Value::Vec2(a) => PyTuple::new(py, a).unwrap().into_any().unbind(),
        Value::Vec3(a) => PyTuple::new(py, a).unwrap().into_any().unbind(),
        Value::Vec4(a) => PyTuple::new(py, a).unwrap().into_any().unbind(),
    }
}

/// Convert a Python result back to a parameter value.
fn py_to_value(obj: &Bound<'_, PyAny>) -> Result<Value, String> {
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

fn format_console_value(v: &Value) -> String {
    match v {
        Value::Float(f) => format!("{f}"),
        Value::Int(i) => format!("{i}"),
        Value::Bool(b) => format!("{b}"),
        Value::Str(s) => s.clone(),
        Value::Vec2(a) => format!("({:.4}, {:.4})", a[0], a[1]),
        Value::Vec3(a) => format!("({:.4}, {:.4}, {:.4})", a[0], a[1], a[2]),
        Value::Vec4(a) => format!(
            "({:.4}, {:.4}, {:.4}, {:.4})",
            a[0], a[1], a[2], a[3]
        ),
    }
}

fn merge_console_out(captured: &str, v: &Value) -> String {
    let formatted = format_console_value(v);
    if captured.is_empty() {
        formatted
    } else if formatted.is_empty() {
        captured.to_string()
    } else {
        format!("{captured}\n{formatted}")
    }
}

fn drain_captured_stdout(_py: Python<'_>, cap: &Bound<'_, PyAny>) -> String {
    cap.call_method0("getvalue")
        .and_then(|v| v.extract())
        .unwrap_or_default()
}

fn restore_stdio(py: Python<'_>, sys: &Bound<'_, PyModule>, out: Py<PyAny>, err: Py<PyAny>) {
    let _ = sys.setattr("stdout", out.bind(py));
    let _ = sys.setattr("stderr", err.bind(py));
}

fn capture_stdio(
    py: Python<'_>,
) -> Result<(Bound<'_, pyo3::types::PyModule>, Py<PyAny>, Py<PyAny>, Bound<'_, PyAny>), String> {
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

pub struct PyEngine {
    /// Names available to every expression, built once.
    globals: Py<PyDict>,
    /// Compiled expressions, keyed by source.
    cache: HashMap<String, Py<PyAny>>,
    /// One namespace per callback source, so the `def`s in an Execute DAT are
    /// executed once rather than on every frame that fires one of them. Keyed
    /// by source, so editing the script rebuilds exactly that namespace — and
    /// module-level state written by a callback survives between frames, which
    /// is what makes a counter in an Execute DAT work at all.
    namespaces: HashMap<String, Py<PyDict>>,
    /// Modules the user has asked for, kept so `import` costs happen once.
    pub startup_error: Option<String>,
    script_log: Vec<String>,
}

impl PyEngine {
    /// Start the interpreter and build the expression scope.
    ///
    /// Returns an engine with `startup_error` set rather than failing: a
    /// project that uses no Python must still open on a machine where the
    /// interpreter cannot start.
    pub fn new() -> PyEngine {
        otd_core::python_runtime::configure_bundled_python();
        let mut engine = PyEngine {
            globals: Python::attach(|py| PyDict::new(py).unbind()),
            cache: HashMap::new(),
            namespaces: HashMap::new(),
            startup_error: None,
            script_log: Vec::new(),
        };
        if let Err(e) = engine.build_scope() {
            engine.startup_error = Some(e);
        }
        engine
    }

    fn build_scope(&mut self) -> Result<(), String> {
        Python::attach(|py| {
            let globals = self.globals.bind(py);
            // A small, useful standard library rather than everything: these
            // are what expressions actually reach for.
            for module in ["math", "random", "json"] {
                let m = py
                    .import(module)
                    .map_err(|e| format!("import {module}: {}", format_error(py, &e)))?;
                globals.set_item(module, m).map_err(|e| e.to_string())?;
            }
            let math = py.import("math").map_err(|e| e.to_string())?;
            for name in [
                "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "sqrt", "floor", "ceil",
                "exp", "log", "pi", "tau", "fmod", "hypot", "degrees", "radians",
            ] {
                if let Ok(f) = math.getattr(name) {
                    globals.set_item(name, f).map_err(|e| e.to_string())?;
                }
            }
            globals
                .set_item("ch", wrap_pyfunction!(ch, py).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            globals
                .set_item("op", wrap_pyfunction!(op, py).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            globals
                .set_item("par", wrap_pyfunction!(par, py).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            globals
                .set_item(
                    "parent",
                    wrap_pyfunction!(parent, py).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            globals
                .set_item(
                    "setpar",
                    wrap_pyfunction!(setpar, py).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
            globals
                .set_item("mod", wrap_pyfunction!(mod_op, py).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            // Helpers with no Python equivalent that artists expect.
            py.run(
                c"
def clamp(v, lo, hi):
    return lo if v < lo else (hi if v > hi else v)

def fit(v, oldlo, oldhi, newlo, newhi):
    t = (v - oldlo) / (oldhi - oldlo) if oldhi != oldlo else 0.0
    return newlo + t * (newhi - newlo)

def lerp(a, b, t):
    return a + (b - a) * t

def smoothstep(lo, hi, v):
    t = clamp((v - lo) / (hi - lo) if hi != lo else 0.0, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)
",
                Some(globals),
                None,
            )
            .map_err(|e| format!("scope setup: {}", format_error(py, &e)))?;
            Ok(())
        })
    }

    /// Whether the interpreter is usable.
    pub fn is_available(&self) -> bool {
        self.startup_error.is_none()
    }

    pub fn take_script_log(&mut self) -> Vec<String> {
        std::mem::take(&mut self.script_log)
    }

    fn push_script_output(&mut self, path: &str, text: String) {
        if text.is_empty() {
            return;
        }
        let tag = path.trim();
        for line in text.lines() {
            if line.is_empty() {
                continue;
            }
            if tag.is_empty() {
                self.script_log.push(line.to_string());
            } else {
                self.script_log.push(format!("{tag}: {line}"));
            }
        }
    }

    /// Evaluate one expression.
    ///
    /// `path` is the operator being cooked, so `parent(...)` resolves against
    /// the right component.
    pub fn eval(&mut self, source: &str, ctx: &EvalContext, path: &str) -> Result<Value, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        let compiled = self.compile(source)?;
        let _guard = CallbackGuard::set(ctx.channels, path);

        let v = Python::attach(|py| {
            let globals = self.globals.bind(py);
            let locals = PyDict::new(py);
            locals.set_item("absTime", ctx.abs_time).ok();
            locals.set_item("time", ctx.time).ok();
            locals.set_item("frame", ctx.frame).ok();
            locals.set_item("fps", ctx.fps).ok();
            locals.set_item("me", me_op_ref()).ok();

            let builtins = py.import("builtins").map_err(|e| format_error(py, &e))?;
            let result = builtins
                .call_method1("eval", (compiled.bind(py), globals.clone(), locals))
                .map_err(|e| format_error(py, &e))?;
            py_to_value(&result)
        })?;
        after_script_invoke()?;
        Ok(v)
    }

    fn compile(&mut self, source: &str) -> Result<Py<PyAny>, String> {
        if let Some(c) = self.cache.get(source) {
            return Ok(Python::attach(|py| c.clone_ref(py)));
        }
        let code = Python::attach(|py| {
            let src = CString::new(source).map_err(|_| "expression contains a NUL byte")?;
            let builtins = py.import("builtins").map_err(|e| format_error(py, &e))?;
            let code = builtins
                .call_method1("compile", (src.to_str().unwrap(), "<parameter>", "eval"))
                .map_err(|e| format_error(py, &e))?;
            Ok::<Py<PyAny>, String>(code.unbind())
        })?;
        // Bound in size so a project that rewrites expressions procedurally
        // cannot grow the cache without limit.
        if self.cache.len() > 4096 {
            self.cache.clear();
        }
        self.cache.insert(source.to_string(), code);
        Ok(Python::attach(|py| self.cache[source].clone_ref(py)))
    }

    pub fn exec_console(
        &mut self,
        source: &str,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<String, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        let _guard = CallbackGuard::set(ctx.channels, path);
        Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let value = (|| -> Result<Option<Value>, String> {
                let globals = self.globals.bind(py);
                let locals = PyDict::new(py);
                locals.set_item("absTime", ctx.abs_time).ok();
                locals.set_item("time", ctx.time).ok();
                locals.set_item("frame", ctx.frame).ok();
                locals.set_item("me", me_op_ref()).ok();
                let src = CString::new(source).map_err(|_| "script contains a NUL byte")?;
                let builtins = py.import("builtins").map_err(|e| format_error(py, &e))?;
                let compiled = builtins.call_method1(
                    "compile",
                    (src.to_str().unwrap(), "<textport>", "eval"),
                );
                if let Ok(code) = compiled {
                    let result = builtins
                        .call_method1("eval", (code, globals.clone(), locals))
                        .map_err(|e| format_error(py, &e))?;
                    if result.is_none() {
                        return Ok(None);
                    }
                    return py_to_value(&result).map(Some);
                }
                py.run(&src, Some(globals), Some(&locals))
                    .map_err(|e| format_error(py, &e))?;
                match locals.get_item("result") {
                    Ok(Some(v)) => py_to_value(&v).map(Some),
                    _ => Ok(None),
                }
            })();
            let captured = drain_captured_stdout(py, &cap).trim_end().to_string();
            restore_stdio(py, &sys, old_out, old_err);
            match value {
                Err(e) => Err(e),
                Ok(None) => Ok(captured),
                Ok(Some(v)) => Ok(merge_console_out(&captured, &v)),
            }
        })
    }

    /// Run a statement block — a Script DAT, or a callback body. Returns
    /// whatever the block left in a variable called `result`, if anything.
    pub fn run(
        &mut self,
        source: &str,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<Option<Value>, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        let _guard = CallbackGuard::set(ctx.channels, path);
        let out = Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let run: Result<Option<Value>, String> = (|| {
                let globals = self.globals.bind(py);
                let locals = PyDict::new(py);
                locals.set_item("absTime", ctx.abs_time).ok();
                locals.set_item("time", ctx.time).ok();
                locals.set_item("frame", ctx.frame).ok();
                locals.set_item("me", me_op_ref()).ok();
                let src = CString::new(source).map_err(|_| "script contains a NUL byte")?;
                py.run(&src, Some(globals), Some(&locals))
                    .map_err(|e| format_error(py, &e))?;
                match locals.get_item("result") {
                    Ok(Some(v)) => py_to_value(&v).map(Some),
                    _ => Ok(None),
                }
            })();
            let captured = drain_captured_stdout(py, &cap);
            restore_stdio(py, &sys, old_out, old_err);
            self.push_script_output(path, captured);
            run
        })?;
        after_script_invoke()?;
        Ok(out)
    }

    /// Call a function defined in `source`, if it defines one by that name.
    ///
    /// Returns whether the function existed, so an Execute DAT can leave the
    /// callbacks it does not care about undefined rather than having to write
    /// an empty body for each.
    pub fn call(
        &mut self,
        source: &str,
        func: &str,
        args: &[Value],
        ctx: &EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        let _guard = CallbackGuard::set(ctx.channels, path);
        // Rebuild the namespace only when the source changed.
        if !self.namespaces.contains_key(source) {
            let built = Python::attach(|py| -> Result<Py<PyDict>, String> {
                let ns = PyDict::new(py);
                let src = CString::new(source).map_err(|_| "script contains a NUL byte")?;
                py.run(&src, Some(self.globals.bind(py)), Some(&ns))
                    .map_err(|e| format_error(py, &e))?;
                Ok(ns.unbind())
            })?;
            // One namespace per Execute DAT in a project is the realistic
            // count; this bound is only here so a script being retyped does
            // not accumulate one per keystroke.
            if self.namespaces.len() > 64 {
                self.namespaces.clear();
            }
            self.namespaces.insert(source.to_string(), built);
        }

        let out = Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let run: Result<bool, String> = (|| {
                let ns = self.namespaces[source].bind(py);
                let Ok(Some(f)) = ns.get_item(func) else {
                    return Ok(false);
                };
                if !f.is_callable() {
                    return Ok(false);
                }
                ns.set_item("me", me_op_ref()).ok();
                ns.set_item("absTime", ctx.abs_time).ok();
                ns.set_item("time", ctx.time).ok();
                ns.set_item("frame", ctx.frame).ok();

                let args: Vec<Py<PyAny>> = args.iter().map(|v| value_to_py(py, v)).collect();
                let tuple = PyTuple::new(py, &args).map_err(|e| format_error(py, &e))?;
                f.call1(tuple).map_err(|e| format_error(py, &e))?;
                Ok(true)
            })();
            let captured = drain_captured_stdout(py, &cap);
            restore_stdio(py, &sys, old_out, old_err);
            self.push_script_output(path, captured);
            run
        })?;
        after_script_invoke()?;
        Ok(out)
    }

    fn ensure_namespace(&mut self, source: &str) -> Result<(), String> {
        if self.namespaces.contains_key(source) {
            return Ok(());
        }
        let built = Python::attach(|py| -> Result<Py<PyDict>, String> {
            let ns = PyDict::new(py);
            let src = CString::new(source).map_err(|_| "script contains a NUL byte")?;
            py.run(&src, Some(self.globals.bind(py)), Some(&ns))
                .map_err(|e| format_error(py, &e))?;
            Ok(ns.unbind())
        })?;
        if self.namespaces.len() > 64 {
            self.namespaces.clear();
        }
        self.namespaces.insert(source.to_string(), built);
        Ok(())
    }

    fn make_callback_op(&self, py: Python<'_>, op_name: &str) -> PyResult<Py<PyAny>> {
        OpRef {
            path: op_name.to_string(),
        }
        .into_pyobject(py)
        .map(|o| o.into_any().unbind())
    }

    pub fn call_webserver_event(
        &mut self,
        source: &str,
        func: &str,
        dat_name: &str,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        self.ensure_namespace(source)?;
        let _guard = CallbackGuard::set(ctx.channels, path);
        let out = Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let run: Result<bool, String> = (|| {
                let ns = self.namespaces[source].bind(py);
                let Ok(Some(f)) = ns.get_item(func) else {
                    return Ok(false);
                };
                if !f.is_callable() {
                    return Ok(false);
                }
                ns.set_item("me", me_op_ref()).ok();
                let dat = self
                    .make_callback_op(py, dat_name)
                    .map_err(|e| format_error(py, &e))?;
                f.call1((dat,)).map_err(|e| format_error(py, &e))?;
                Ok(true)
            })();
            let captured = drain_captured_stdout(py, &cap);
            restore_stdio(py, &sys, old_out, old_err);
            self.push_script_output(path, captured);
            run
        })?;
        after_script_invoke()?;
        Ok(out)
    }

    pub fn call_webserver_ws(
        &mut self,
        source: &str,
        func: &str,
        dat_name: &str,
        client: &str,
        uri: Option<&str>,
        text: Option<&str>,
        binary: Option<&[u8]>,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        self.ensure_namespace(source)?;
        let _guard = CallbackGuard::set(ctx.channels, path);
        let out = Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let run: Result<bool, String> = (|| {
                let ns = self.namespaces[source].bind(py);
                let Ok(Some(f)) = ns.get_item(func) else {
                    return Ok(false);
                };
                if !f.is_callable() {
                    return Ok(false);
                }
                ns.set_item("me", me_op_ref()).ok();
                let dat = self
                    .make_callback_op(py, dat_name)
                    .map_err(|e| format_error(py, &e))?;
                match (uri, text, binary) {
                    (Some(uri), None, None) => {
                        f.call1((dat, client, uri)).map_err(|e| format_error(py, &e))?;
                    }
                    (None, Some(text), None) => {
                        f.call1((dat, client, text)).map_err(|e| format_error(py, &e))?;
                    }
                    (None, None, Some(bytes)) => {
                        let b = PyBytes::new(py, bytes);
                        f.call1((dat, client, b)).map_err(|e| format_error(py, &e))?;
                    }
                    (None, None, None) => {
                        f.call1((dat, client)).map_err(|e| format_error(py, &e))?;
                    }
                    _ => return Err(format!("bad webserver_ws args for {func}")),
                }
                Ok(true)
            })();
            let captured = drain_captured_stdout(py, &cap);
            restore_stdio(py, &sys, old_out, old_err);
            self.push_script_output(path, captured);
            run
        })?;
        after_script_invoke()?;
        Ok(out)
    }

    pub fn call_webserver_http(
        &mut self,
        source: &str,
        dat_name: &str,
        method: &str,
        uri: &str,
        pars: &HashMap<String, String>,
        client_address: &str,
        server_address: &str,
        data: &str,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<(u16, String, String, Option<String>), String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        self.ensure_namespace(source)?;
        let _guard = CallbackGuard::set(ctx.channels, path);
        let out = Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let run: Result<(u16, String, String, Option<String>), String> = (|| {
                let ns = self.namespaces[source].bind(py);
                let Ok(Some(f)) = ns.get_item("onHTTPRequest") else {
                    return Ok((404, "Not Found".into(), String::new(), None));
                };
                if !f.is_callable() {
                    return Ok((404, "Not Found".into(), String::new(), None));
                }
                ns.set_item("me", me_op_ref()).ok();
                let request = PyDict::new(py);
                request.set_item("method", method).ok();
                request.set_item("uri", uri).ok();
                let qp = PyDict::new(py);
                for (k, v) in pars {
                    qp.set_item(k, v).ok();
                }
                request.set_item("pars", qp).ok();
                request.set_item("clientAddress", client_address).ok();
                request.set_item("serverAddress", server_address).ok();
                request.set_item("data", data).ok();
                let response = PyDict::new(py);
                response.set_item("statusCode", 404).ok();
                response.set_item("statusReason", "Not Found.").ok();
                response.set_item("data", "").ok();
                let dat = self
                    .make_callback_op(py, dat_name)
                    .map_err(|e| format_error(py, &e))?;
                let _ret = f
                    .call1((dat.clone_ref(py), request, response.clone()))
                    .map_err(|e| format_error(py, &e))?;
                let out = response;
                let status = out
                    .get_item("statusCode")
                    .ok()
                    .flatten()
                    .and_then(|v| v.extract::<i64>().ok())
                    .unwrap_or(404)
                    .clamp(100, 599) as u16;
                let reason = out
                    .get_item("statusReason")
                    .ok()
                    .flatten()
                    .and_then(|v| v.extract::<String>().ok())
                    .unwrap_or_else(|| "OK".into());
                let body = out
                    .get_item("data")
                    .ok()
                    .flatten()
                    .and_then(|v| v.extract::<String>().ok())
                    .unwrap_or_default();
                let content_type = out
                    .get_item("content-type")
                    .ok()
                    .flatten()
                    .and_then(|v| v.extract::<String>().ok());
                Ok((status, reason, body, content_type))
            })();
            let captured = drain_captured_stdout(py, &cap);
            restore_stdio(py, &sys, old_out, old_err);
            self.push_script_output(path, captured);
            run
        })?;
        after_script_invoke()?;
        Ok(out)
    }

    /// Take the parameter changes callbacks have asked for since the last
    /// drain. The host applies them between frames — see `otd_core::edit`.
    pub fn take_edits(&mut self) -> Vec<ParamEdit> {
        take_queued_edits()
    }

    pub fn take_recook_requests(&mut self) -> Vec<String> {
        take_queued_deferred_recooks()
    }

    pub fn lint_script(&self, source: &str, filename: &str) -> Vec<PythonLint> {
        Python::attach(|py| {
            lint::lint_script(py, self.startup_error.as_deref(), source, filename)
        })
    }

    /// Rows of text produced by a script, for a Script DAT.
    pub fn run_table(
        &mut self,
        source: &str,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<Vec<Vec<String>>, String> {
        if let Some(e) = &self.startup_error {
            return Err(e.clone());
        }
        let _guard = CallbackGuard::set(ctx.channels, path);
        let out = Python::attach(|py| {
            let (sys, old_out, old_err, cap) = capture_stdio(py)?;
            let run: Result<Vec<Vec<String>>, String> = (|| {
                let globals = self.globals.bind(py);
                let locals = PyDict::new(py);
                locals.set_item("absTime", ctx.abs_time).ok();
                locals.set_item("frame", ctx.frame).ok();
                locals.set_item("me", me_op_ref()).ok();
                let src = CString::new(source).map_err(|_| "script contains a NUL byte")?;
                py.run(&src, Some(globals), Some(&locals))
                    .map_err(|e| format_error(py, &e))?;

                let Ok(Some(rows)) = locals.get_item("rows") else {
                    return Ok(Vec::new());
                };
                let rows = rows
                    .cast::<PyList>()
                    .map_err(|_| "`rows` must be a list of lists".to_string())?
                    .clone();
                let mut out = Vec::with_capacity(rows.len());
                for row in rows.iter() {
                    let cells = match row.try_iter() {
                        Ok(items) => items
                            .filter_map(|i| i.ok())
                            .map(|i| i.str().map(|s| s.to_string()).unwrap_or_default())
                            .collect(),
                        Err(_) => vec![row.str().map(|s| s.to_string()).unwrap_or_default()],
                    };
                    out.push(cells);
                }
                Ok(out)
            })();
            let captured = drain_captured_stdout(py, &cap);
            restore_stdio(py, &sys, old_out, old_err);
            self.push_script_output(path, captured);
            run
        })?;
        after_script_invoke()?;
        Ok(out)
    }
}

impl Default for PyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeNet;
    impl ChannelSource for FakeNet {
        fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
            (op_path == "/lfo1" && channel == "chan1").then_some(0.5)
        }
        fn param_value(&self, op_path: &str, name: &str) -> Option<Value> {
            (op_path == "/blur1" && name == "size").then_some(Value::Float(8.0))
        }
        fn parent_param(&self, _node: &str, name: &str) -> Option<Value> {
            (name == "speed").then_some(Value::Float(2.5))
        }
        fn resolve_operator_path(&self, _from: &str, reference: &str) -> Option<String> {
            match reference {
                "text1" => Some("/text1".into()),
                "table1" => Some("/table1".into()),
                p if p.starts_with('/') => Some(p.into()),
                _ => None,
            }
        }
        fn dat_text(&self, op_path: &str) -> Option<String> {
            (op_path == "/text1").then(|| "from the dat".into())
        }
        fn dat_rows(&self, op_path: &str) -> Option<Vec<Vec<String>>> {
            (op_path == "/table1").then(|| {
                vec![
                    vec!["a".into(), "b".into()],
                    vec!["1".into(), "2".into()],
                ]
            })
        }
    }

    fn ctx() -> EvalContext<'static> {
        EvalContext {
            abs_time: 2.0,
            frame: 120,
            fps: 60.0,
            channels: Some(&FakeNet),
            ..Default::default()
        }
    }

    #[test]
    fn arithmetic_and_the_maths_scope() {
        let mut e = PyEngine::new();
        assert!(e.is_available(), "{:?}", e.startup_error);
        assert_eq!(e.eval("1 + 2 * 3", &ctx(), "/x").unwrap().as_f64(), 7.0);
        assert!((e.eval("sin(pi / 2)", &ctx(), "/x").unwrap().as_f64() - 1.0).abs() < 1e-12);
        assert_eq!(
            e.eval("clamp(5, 0, 1)", &ctx(), "/x").unwrap().as_f64(),
            1.0
        );
        assert_eq!(
            e.eval("fit(0.5, 0, 1, 10, 20)", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            15.0
        );
    }

    #[test]
    fn time_and_the_network_are_in_scope() {
        let mut e = PyEngine::new();
        assert_eq!(e.eval("absTime", &ctx(), "/x").unwrap().as_f64(), 2.0);
        assert_eq!(e.eval("frame", &ctx(), "/x").unwrap().as_i64(), 120);
        assert_eq!(
            e.eval("ch('/lfo1', 'chan1')", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            0.5
        );
        assert_eq!(
            e.eval("par('/blur1', 'size')", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            8.0
        );
        assert_eq!(
            e.eval("parent('speed')", &ctx(), "/x").unwrap().as_f64(),
            2.5
        );
    }

    #[test]
    fn python_expressions_can_use_python() {
        let mut e = PyEngine::new();
        assert_eq!(
            e.eval("sum(x * x for x in range(4))", &ctx(), "/x")
                .unwrap()
                .as_f64(),
            14.0
        );
        assert_eq!(
            e.eval("'on' if absTime > 1 else 'off'", &ctx(), "/x")
                .unwrap()
                .as_str(),
            "on"
        );
        assert_eq!(
            e.eval("[1.0, 2.0, 3.0]", &ctx(), "/x").unwrap(),
            Value::Vec3([1.0, 2.0, 3.0])
        );
    }

    #[test]
    fn op_text_reads_dat_contents() {
        let mut e = PyEngine::new();
        assert_eq!(
            e.eval("op('text1').text", &ctx(), "/webserver1")
                .unwrap()
                .as_str(),
            "from the dat"
        );
    }

    #[test]
    fn op_dat_table_cells() {
        let mut e = PyEngine::new();
        assert_eq!(
            e.eval("op('table1').numRows", &ctx(), "/x")
                .unwrap()
                .as_i64(),
            2
        );
        assert_eq!(
            e.eval("op('table1').getCell(1, 1)", &ctx(), "/x")
                .unwrap()
                .as_str(),
            "2"
        );
        assert_eq!(
            e.eval("op('table1')[1, 1]", &ctx(), "/x").unwrap().as_str(),
            "2"
        );
    }

    #[test]
    fn an_error_is_reported_as_one_line() {
        let mut e = PyEngine::new();
        let err = e.eval("1 / 0", &ctx(), "/x").unwrap_err();
        assert!(err.contains("ZeroDivisionError"), "{err}");
        assert!(err.len() < 200, "{err}");

        let err = e.eval("nope +", &ctx(), "/x").unwrap_err();
        assert!(err.contains("SyntaxError"), "{err}");

        let err = e.eval("undefined_name", &ctx(), "/x").unwrap_err();
        assert!(err.contains("NameError"), "{err}");
    }

    #[test]
    fn the_network_is_unreachable_once_evaluation_ends() {
        let mut e = PyEngine::new();
        // Stash the function, then call it outside any evaluation: it must
        // read nothing rather than dereference a stale pointer.
        e.eval("ch('/lfo1', 'chan1')", &ctx(), "/x").unwrap();
        let no_net = EvalContext {
            channels: None,
            ..Default::default()
        };
        assert_eq!(
            e.eval("ch('/lfo1', 'chan1')", &no_net, "/x")
                .unwrap()
                .as_f64(),
            0.0
        );
    }

    #[test]
    fn a_script_can_return_a_value_and_a_table() {
        let mut e = PyEngine::new();
        let v = e
            .run(
                "total = 0\nfor i in range(5):\n    total += i\nresult = total",
                &ctx(),
                "/x",
            )
            .unwrap();
        assert_eq!(v.map(|v| v.as_f64()), Some(10.0));

        let rows = e
            .run_table(
                "rows = [['name', 'value'], ['a', 1], ['b', 2]]",
                &ctx(),
                "/x",
            )
            .unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["name", "value"]);
        assert_eq!(rows[2][1], "2");
    }

    #[test]
    fn compiling_the_same_source_twice_reuses_it() {
        let mut e = PyEngine::new();
        for _ in 0..100 {
            e.eval("absTime * 2", &ctx(), "/x").unwrap();
        }
        assert_eq!(e.cache.len(), 1);
    }
}
