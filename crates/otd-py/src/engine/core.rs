use super::PyEngine;
use super::*;

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

    pub(crate) fn push_script_output(&mut self, path: &str, text: String) {
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
}

