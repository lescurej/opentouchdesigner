use otd_core::indexmap::IndexMap;
use otd_core::Param;
use otd_core::EvalContext;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "op" => Param::str("..").with_label("Operator").as_path_ref(),
        "parameters" => Param::str("value").with_label("Parameters"),
        "name" => Param::str("v").with_label("Channel Name"),
    }
}

fn resolve_op_path(c: &ChopCtx<'_>, path: &str) -> Option<otd_core::NodeId> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        c.graph.find(path)
    } else {
        c.graph.find_from(c.id, path)
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let watch = match resolve_op_path(c, &c.s("op")) {
        Some(id) => id,
        None => return ChopData::empty(),
    };
    let watch_path = c.graph.path(watch);
    let prefix = c.s("name");
    let names: Vec<String> = c
        .s("parameters")
        .split_whitespace()
        .filter(|n| !n.is_empty() && *n != "*")
        .map(|s| s.to_string())
        .collect();
    let chan_names = otd_core::expand_channel_names(&prefix, names.len());
    let channels: Vec<Channel> = names
        .iter()
        .enumerate()
        .map(|(i, key)| {
            let value = c
                .graph
                .get(watch)
                .and_then(|n| n.param(key))
                .map(|p| {
                    p.eval(&EvalContext {
                        channels: c.eval.channels,
                        path: Some(&watch_path),
                        ..*c.eval
                    })
                    .as_f32()
                })
                .unwrap_or(0.0);
            let chan = chan_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("{prefix}{}", i + 1));
            Channel::constant(chan, value, 1)
        })
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}


pub const TYPE_NAME: &str = "parameterCHOP";
pub const PARAMETER: &str = TYPE_NAME;

pub fn spec() -> ChopSpec {
    super::spec(
        TYPE_NAME,
        "Parameter",
        &[],
        "Another operator's parameter values as channels (does not cook that operator).",
        params,
        cook,
    )
}
