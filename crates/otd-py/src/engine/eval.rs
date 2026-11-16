use super::PyEngine;
use super::*;

impl PyEngine {
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


}
