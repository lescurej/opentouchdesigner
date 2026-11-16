use super::PyEngine;
use super::*;

impl PyEngine {
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
