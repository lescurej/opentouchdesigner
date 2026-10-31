//! `otd-dat` — the DAT (data operator) family: text and tables.
//!
//! PLAN.md §2.1 lists DATs alongside TOPs and CHOPs as one of the load-bearing
//! wire families. They are the family that carries *structure* — a table of
//! cue points, a JSON payload from a control surface, a script.
//!
//! Two decisions worth stating. Content lives in a parameter rather than in a
//! side file, so a table is part of the project's text and shows up in a diff
//! like everything else. And a table is a grid of strings, not of typed
//! values: a DAT is text, and the operator reading it decides what the text
//! means.

pub mod callbacks;
pub mod net;
pub mod ops;
pub mod udp;
pub mod webserver;

use otd_core::{
    ChannelSource, CookContext, CookError, Cooker, Crossings, EvalContext, Family, Graph, NodeId,
};
use slotmap::SecondaryMap;

/// A DAT's contents: rows of cells.
///
/// A "text" DAT is simply one cell; keeping one representation means Select,
/// Merge and the viewers do not need two code paths.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DatData {
    pub rows: Vec<Vec<String>>,
    /// Presented as free text rather than as a grid.
    pub is_text: bool,
}

impl DatData {
    pub fn text(s: impl Into<String>) -> Self {
        DatData {
            rows: vec![vec![s.into()]],
            is_text: true,
        }
    }

    pub fn table(rows: Vec<Vec<String>>) -> Self {
        DatData {
            rows,
            is_text: false,
        }
    }

    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn num_cols(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    pub fn cell(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// The whole thing as text — a text DAT's single cell, or the table as
    /// tab-separated rows.
    pub fn as_text(&self) -> String {
        if self.is_text {
            return self.cell(0, 0).to_string();
        }
        self.rows
            .iter()
            .map(|r| r.join("\t"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse tab- or comma-separated text into a table.
    pub fn from_delimited(text: &str, delimiter: char) -> Self {
        let rows: Vec<Vec<String>> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split(delimiter).map(|c| c.trim().to_string()).collect())
            .collect();
        DatData::table(rows)
    }

    /// Look a value up by its row heading, the way a cue table is used.
    pub fn lookup(&self, key: &str, column: usize) -> Option<&str> {
        self.rows
            .iter()
            .find(|r| r.first().map(|c| c == key).unwrap_or(false))
            .and_then(|r| r.get(column))
            .map(|s| s.as_str())
    }

    pub fn set_cell(&mut self, row: usize, col: usize, value: impl Into<String>) {
        while self.rows.len() <= row {
            self.rows.push(Vec::new());
        }
        while self.rows[row].len() <= col {
            self.rows[row].push(String::new());
        }
        self.rows[row][col] = value.into();
        if self.is_text && (row > 0 || col > 0 || self.rows.len() > 1) {
            self.is_text = false;
        }
    }

    pub fn clear_grid(&mut self) {
        self.rows.clear();
        if self.is_text {
            self.rows.push(vec![String::new()]);
        }
    }

    pub fn append_row(&mut self, cells: &[impl AsRef<str>]) {
        self.is_text = false;
        self.rows
            .push(cells.iter().map(|c| c.as_ref().to_string()).collect());
    }

    pub fn append_col(&mut self, default: &str) {
        self.is_text = false;
        let target = self.num_cols().max(1);
        if self.rows.is_empty() {
            self.rows.push(vec![default.to_string()]);
            return;
        }
        for row in &mut self.rows {
            while row.len() < target {
                row.push(String::new());
            }
            row.push(default.to_string());
        }
    }

    pub fn insert_row(&mut self, index: usize, cells: &[impl AsRef<str>]) {
        self.is_text = false;
        let row: Vec<String> = cells.iter().map(|c| c.as_ref().to_string()).collect();
        if index >= self.rows.len() {
            self.rows.push(row);
        } else {
            self.rows.insert(index, row);
        }
    }

    pub fn delete_row(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
        }
    }

    pub fn to_param_text(&self) -> String {
        self.as_text()
    }

    pub fn apply_cell_write(grid: &mut Vec<Vec<String>>, row: usize, col: usize, value: &str) {
        while grid.len() <= row {
            grid.push(Vec::new());
        }
        while grid[row].len() <= col {
            grid[row].push(String::new());
        }
        grid[row][col] = value.to_string();
    }
}

/// Every DAT's most recent output.
#[derive(Default)]
pub struct DatStore {
    data: SecondaryMap<NodeId, DatData>,
}

impl DatStore {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn get(&self, id: NodeId) -> Option<&DatData> {
        self.data.get(id)
    }
    pub fn insert(&mut self, id: NodeId, data: DatData) {
        self.data.insert(id, data);
    }
    pub fn remove(&mut self, id: NodeId) {
        self.data.remove(id);
    }
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

/// What a Script DAT needs from the host to run its source.
pub trait ScriptHost {
    /// Run a script and collect the `rows` it leaves behind.
    fn run_table(
        &self,
        source: &str,
        ctx: &EvalContext,
        path: &str,
    ) -> Result<Vec<Vec<String>>, String>;

    /// Call one callback defined in `source`. `Ok(false)` means the script
    /// simply does not define it, which is the normal case — an Execute DAT
    /// implements the events it cares about and leaves the rest out.
    fn call(
        &self,
        _source: &str,
        _func: &str,
        _args: &[otd_core::Value],
        _ctx: &EvalContext,
        _path: &str,
    ) -> Result<bool, String> {
        Ok(false)
    }

    fn webserver_http(
        &self,
        _source: &str,
        _dat_name: &str,
        _job: &webserver::HttpJob,
        _ctx: &EvalContext,
        _path: &str,
    ) -> Result<webserver::HttpReply, String> {
        Err("no Python interpreter in this build".into())
    }

    fn webserver_event(
        &self,
        _source: &str,
        _func: &str,
        _dat_name: &str,
        _ctx: &EvalContext,
        _path: &str,
    ) -> Result<bool, String> {
        Ok(false)
    }

    fn webserver_ws(
        &self,
        _source: &str,
        _func: &str,
        _dat_name: &str,
        _client: &str,
        _uri: Option<&str>,
        _text: Option<&str>,
        _binary: Option<&[u8]>,
        _ctx: &EvalContext,
        _path: &str,
    ) -> Result<bool, String> {
        Ok(false)
    }
}

#[derive(Default)]
pub struct DatEngine {
    /// Errors from Script DATs, shown on the node.
    pub status: std::collections::HashMap<String, String>,
    /// Open sockets for the network DATs, keyed by node path.
    pub net: net::Net,
    /// What each Execute DAT saw last frame, so it can tell what changed.
    /// Keyed by path: renaming one resets its memory, which is the right
    /// answer anyway — a renamed node is a different node to a callback.
    pub watched: std::collections::HashMap<String, ops::Watched>,
}

impl DatEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status_of(&self, path: &str) -> Option<&str> {
        self.status.get(path).map(|s| s.as_str())
    }

    pub fn reset(&mut self) {
        self.status.clear();
        self.net.reset();
    }

    pub fn apply_udp_sends(&mut self, _graph: &Graph, sends: &[(String, Vec<u8>)]) -> usize {
        let mut n = 0;
        for (path, data) in sends {
            let Some(out) = self.net.udp_out.get_mut(path) else {
                continue;
            };
            if out.listener.send_to(data, out.target).is_ok() {
                out.last_sent = Some(String::from_utf8_lossy(data).into_owned());
                n += 1;
            }
        }
        n
    }

    /// Cook one DAT and return its contents.
    #[allow(clippy::too_many_arguments)]
    pub fn cook_node(
        &mut self,
        graph: &Graph,
        id: NodeId,
        _ctx: &CookContext,
        eval: &EvalContext,
        store: &DatStore,
        scripts: Option<&dyn ScriptHost>,
        foreign: Crossings,
    ) -> Result<DatData, CookError> {
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let path = graph.path(id);

        if node.connector == otd_core::Connector::In {
            return Ok(graph
                .connector_source(id)
                .and_then(|src| store.get(src))
                .cloned()
                .unwrap_or_default());
        }

        let spec = ops::spec_for(&node.op_type)
            .ok_or_else(|| CookError::op(&path, format!("unknown DAT `{}`", node.op_type)))?;

        let inputs: Vec<DatData> = node
            .inputs
            .iter()
            .map(|slot| {
                slot.and_then(|src| store.get(src))
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();

        let eval = &EvalContext {
            path: Some(&path),
            ..*eval
        };
        let mut watched = self.watched.remove(&path).unwrap_or_default();
        let mut cx = ops::DatCtx {
            graph,
            id,
            node,
            eval,
            inputs,
            foreign,
            scripts,
            net: &mut self.net,
            watched: &mut watched,
            path: &path,
            error: None,
        };
        let out = (spec.cook)(&mut cx);
        let error = cx.error.take();
        self.watched.insert(path.clone(), watched);
        match error {
            Some(e) => {
                self.status.insert(path, e);
            }
            None => {
                self.status.remove(&path);
            }
        }
        Ok(out)
    }
}

/// A DAT-only host, for tests and for a control-only runtime.
#[derive(Default)]
pub struct DatHost {
    pub engine: DatEngine,
    pub store: DatStore,
}

impl DatHost {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn data(&self, id: NodeId) -> Option<&DatData> {
        self.store.get(id)
    }
}

impl Cooker for DatHost {
    fn cook(&mut self, graph: &Graph, id: NodeId, ctx: &CookContext) -> Result<otd_core::CookOutput, CookError> {
        if graph.get(id).map(|n| n.family) != Some(Family::Dat) {
            return Ok(otd_core::CookOutput::Updated);
        }
        let DatHost { engine, store } = self;
        let eval = ctx.eval_ctx();
        let data = engine.cook_node(graph, id, ctx, &eval, store, None, Crossings::new())?;
        store.insert(id, data);
        Ok(otd_core::CookOutput::Updated)
    }
}

/// A read-only view of DAT contents, so a parameter can read a table cell.
pub struct DatNetwork<'a> {
    pub graph: &'a Graph,
    pub dats: &'a DatStore,
}

impl ChannelSource for DatNetwork<'_> {
    fn channel(&self, _op_path: &str, _channel: &str) -> Option<f32> {
        None
    }
    fn param_value(&self, _op_path: &str, _param: &str) -> Option<otd_core::Value> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_table_round_trips_through_delimited_text() {
        let d = DatData::from_delimited("name\tvalue\na\t1\nb\t2", '\t');
        assert_eq!(d.num_rows(), 3);
        assert_eq!(d.num_cols(), 2);
        assert_eq!(d.cell(1, 0), "a");
        assert_eq!(d.lookup("b", 1), Some("2"));
        assert_eq!(d.as_text(), "name\tvalue\na\t1\nb\t2");
    }

    #[test]
    fn ragged_rows_read_as_empty_rather_than_panicking() {
        let d = DatData::table(vec![vec!["a".into()], vec!["b".into(), "c".into()]]);
        assert_eq!(d.num_cols(), 2);
        assert_eq!(d.cell(0, 1), "");
        assert_eq!(d.cell(9, 9), "");
    }

    #[test]
    fn a_text_dat_is_one_cell() {
        let d = DatData::text("hello\nworld");
        assert!(d.is_text);
        assert_eq!(d.num_rows(), 1);
        assert_eq!(d.as_text(), "hello\nworld");
    }
}
