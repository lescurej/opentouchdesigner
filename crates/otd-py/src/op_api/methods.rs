use super::*;

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
