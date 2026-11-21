//! FullNetwork and DAT helpers for parameter evaluation.

use std::cell::RefCell;

use otd_chop::engine::Network;
use otd_core::{Family, Graph};
use otd_dat::DatStore;
use otd_py::PyEngine;

use otd_dat::DatData;

pub(crate) struct FullNetwork<'a> {
    pub(crate) chops: Network<'a>,
    pub(crate) dats: &'a DatStore,
    pub(crate) python: &'a RefCell<PyEngine>,
}

pub fn dat_data_at(graph: &Graph, dats: &DatStore, op_path: &str) -> Option<DatData> {
    let id = graph.find(op_path.trim())?;
    if let Some(data) = dats.get(id) {
        return Some(data.clone());
    }
    let node = graph.get(id)?;
    if node.family != Family::Dat {
        return None;
    }
    match node.op_type.as_str() {
        otd_dat::ops::TABLE => {
            let t = node.param("text")?.value.as_str();
            Some(DatData::from_delimited(&t, '\t'))
        }
        otd_dat::ops::TEXT => {
            let t = node.param("text")?.value.as_str();
            Some(DatData::text(t))
        }
        otd_dat::ops::SCRIPT => {
            let t = node.param("source")?.value.as_str();
            Some(DatData::text(t))
        }
        _ => {
            if let Some(p) = node.param("text") {
                let t = p.value.as_str();
                if t.contains('\t') || t.contains('\n') {
                    Some(DatData::from_delimited(&t, '\t'))
                } else {
                    Some(DatData::text(t))
                }
            } else {
                None
            }
        }
    }
}

pub(crate) fn dat_text_at(graph: &Graph, dats: &DatStore, op_path: &str) -> Option<String> {
    dat_data_at(graph, dats, op_path).map(|d| d.as_text())
}

pub(crate) fn dat_content_param_at(graph: &Graph, op_path: &str) -> &'static str {
    graph
        .find(op_path.trim())
        .and_then(|id| graph.get(id))
        .map(|n| {
            if n.op_type.as_str() == otd_dat::ops::SCRIPT {
                "source"
            } else {
                "text"
            }
        })
        .unwrap_or("text")
}

pub(crate) fn dat_rows_at(
    graph: &Graph,
    dats: &DatStore,
    op_path: &str,
) -> Option<Vec<Vec<String>>> {
    dat_data_at(graph, dats, op_path).map(|d| d.rows)
}

impl otd_core::ChannelSource for FullNetwork<'_> {
    fn channel(&self, op_path: &str, channel: &str) -> Option<f32> {
        self.chops.channel(op_path, channel)
    }
    fn channel_names(&self, op_path: &str) -> Vec<String> {
        self.chops.channel_names(op_path)
    }
    fn param_value(&self, op_path: &str, param: &str) -> Option<otd_core::Value> {
        self.chops.param_value(op_path, param)
    }
    fn parent_param(&self, node_path: &str, param: &str) -> Option<otd_core::Value> {
        self.chops.parent_param(node_path, param)
    }
    fn resolve_operator_path(&self, from_operator_path: &str, reference: &str) -> Option<String> {
        self.chops
            .resolve_operator_path(from_operator_path, reference)
    }
    fn eval_python(
        &self,
        source: &str,
        ctx: &otd_core::EvalContext,
        path: &str,
    ) -> Option<Result<otd_core::Value, String>> {
        // A re-entrant evaluation — Python reading a parameter that is itself
        // Python — would deadlock on the RefCell. Refusing is the honest
        // outcome, and the parameter falls back to its constant.
        let mut py = self.python.try_borrow_mut().ok()?;
        Some(py.eval(source, ctx, path))
    }
    fn dat_text(&self, op_path: &str) -> Option<String> {
        dat_text_at(self.chops.graph, self.dats, op_path)
    }
    fn dat_rows(&self, op_path: &str) -> Option<Vec<Vec<String>>> {
        dat_rows_at(self.chops.graph, self.dats, op_path)
    }
    fn dat_content_param(&self, op_path: &str) -> Option<&'static str> {
        self.chops.graph.find(op_path.trim())?;
        Some(dat_content_param_at(self.chops.graph, op_path))
    }

    fn op_type(&self, op_path: &str) -> Option<String> {
        let id = self.chops.graph.find(op_path.trim())?;
        self.chops.graph.get(id).map(|n| n.op_type.clone())
    }
}
