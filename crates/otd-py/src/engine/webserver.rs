use super::PyEngine;
use super::*;

impl PyEngine {
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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


}
