use otd_core::{
    CookContext, CookError, Crossings, EvalContext, Graph, NodeId,
};

use super::data::DatData;
use super::store::DatStore;
use crate::net;
use crate::ops;
use crate::webserver;

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

    #[allow(clippy::too_many_arguments)]
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
        input_keys: Option<&[String]>,
        chop_table_src: Option<(String, usize, usize)>,
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
            input_keys,
            chop_table_src,
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
