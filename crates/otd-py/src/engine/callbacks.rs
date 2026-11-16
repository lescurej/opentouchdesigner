use super::PyEngine;
use super::*;

impl PyEngine {
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

    pub(crate) fn ensure_namespace(&mut self, source: &str) -> Result<(), String> {
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

    pub(crate) fn make_callback_op(&self, py: Python<'_>, op_name: &str) -> PyResult<Py<PyAny>> {
        OpRef {
            path: op_name.to_string(),
        }
        .into_pyobject(py)
        .map(|o| o.into_any().unbind())
    }


}
