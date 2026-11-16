use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Param, Value};

use crate::DatData;

use super::callbacks::{callback_dat_view, fire, resolve_dat_op};
use super::constants::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "op" => Param::str("").with_label("OPs").as_path_ref(),
        "parameters" => Param::str("*").with_label("Parameters"),
        "valuechange" => Param::bool(true).with_label("Value Change"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(PAR_EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    if !c.b("valuechange") {
        return callback_dat_view(c, &source);
    }
    let Some(watch_id) = resolve_dat_op(c, &c.s("op")) else {
        return callback_dat_view(c, &source);
    };
    let watch_path = c.graph.path(watch_id);
    let pattern = c.s("parameters");
    let names: Vec<String> = pattern
        .split_whitespace()
        .filter(|n| *n != "*")
        .map(|s| s.to_string())
        .collect();
    if names.is_empty() {
        c.error = Some("name the parameters to watch — `*` cannot be enumerated".into());
        return callback_dat_view(c, &source);
    }

    let mut events = Vec::new();
    for name in &names {
        let Some(value) = c
            .graph
            .get(watch_id)
            .and_then(|n| n.param(name))
            .map(|p| {
                p.eval(&EvalContext {
                    channels: c.eval.channels,
                    path: Some(&watch_path),
                    ..*c.eval
                })
            })
        else {
            continue;
        };
        let value = value.as_f64();
        let prev = c.watched.values.get(name).copied();
        if prev != Some(value) {
            events.push((name.clone(), value, prev));
        }
    }
    for (name, value, _) in &events {
        c.watched.values.insert(name.clone(), *value);
    }
    for (name, value, prev) in events {
        fire(
            c,
            &source,
            "onValueChange",
            &[
                Value::Str(name),
                Value::Float(value),
                Value::Float(prev.unwrap_or(value)),
            ],
        );
    }
    callback_dat_view(c, &source)
}
pub const TYPE_NAME: &str = "parameterexecuteDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Parameter Execute",
        &[],
        "Parameter change callbacks.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
