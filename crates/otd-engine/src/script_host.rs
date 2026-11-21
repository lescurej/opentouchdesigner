//! Python Script DAT host.

use std::cell::RefCell;

use otd_dat::ScriptHost;
use otd_py::PyEngine;

pub(crate) struct PythonScripts<'a> {
    pub(crate) python: &'a RefCell<PyEngine>,
}

impl ScriptHost for PythonScripts<'_> {
    fn call(
        &self,
        source: &str,
        func: &str,
        args: &[otd_core::Value],
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?
            .call(source, func, args, ctx, path)
    }

    fn run_table(
        &self,
        source: &str,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<Vec<Vec<String>>, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a script cannot run inside another script".to_string())?
            .run_table(source, ctx, path)
    }

    fn webserver_http(
        &self,
        source: &str,
        dat_name: &str,
        job: &otd_dat::webserver::HttpJob,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<otd_dat::webserver::HttpReply, String> {
        let mut py = self
            .python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?;
        let (status, reason, body, content_type) = py.call_webserver_http(
            source,
            dat_name,
            &job.method,
            &job.uri,
            &job.pars,
            &job.client_address,
            &job.server_address,
            &job.data,
            ctx,
            path,
        )?;
        Ok(otd_dat::webserver::HttpReply {
            status,
            reason,
            body,
            content_type,
        })
    }

    fn webserver_event(
        &self,
        source: &str,
        func: &str,
        dat_name: &str,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?
            .call_webserver_event(source, func, dat_name, ctx, path)
    }

    fn webserver_ws(
        &self,
        source: &str,
        func: &str,
        dat_name: &str,
        client: &str,
        uri: Option<&str>,
        text: Option<&str>,
        binary: Option<&[u8]>,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Result<bool, String> {
        self.python
            .try_borrow_mut()
            .map_err(|_| "a callback cannot run inside another script".to_string())?
            .call_webserver_ws(source, func, dat_name, client, uri, text, binary, ctx, path)
    }
}
