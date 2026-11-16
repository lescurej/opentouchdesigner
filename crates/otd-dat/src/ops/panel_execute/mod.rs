use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Param, Value};

use crate::DatData;

use super::callbacks::{callback_dat_view, dat_script_path, fire_at, resolve_dat_op};
use super::constants::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "panels" => Param::str("..").with_label("Panels").as_path_ref(),
        "panelvalue" => Param::str("value").with_label("Panel Value"),
        "offtoon" => Param::bool(true).with_label("Off to On"),
        "ontooff" => Param::bool(true).with_label("On to Off"),
        "valuechange" => Param::bool(true).with_label("Value Change"),
        "whileon" => Param::bool(false).with_label("While On"),
        "whileoff" => Param::bool(false).with_label("While Off"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(PANEL_EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    let Some(panel_id) = resolve_dat_op(c, &c.s("panels")) else {
        return callback_dat_view(c, &source);
    };
    let panel_path = c.graph.path(panel_id);
    let key = c.s("panelvalue");
    let value = c
        .graph
        .get(panel_id)
        .and_then(|n| n.param(&key))
        .map(|p| {
            p.eval(&EvalContext {
                channels: c.eval.channels,
                path: Some(&panel_path),
                ..*c.eval
            })
            .as_f64()
        })
        .unwrap_or(0.0);
    let prev = c.watched.values.get("panel").copied();
    let on = value > 0.0;
    let script = dat_script_path(c);
    let pv = Value::Str(key.clone());

    if let Some(prev_val) = prev {
        let was_on = prev_val > 0.0;
        if c.b("offtoon") && !was_on && on {
            fire_at(c, &source, "onOffToOn", std::slice::from_ref(&pv), &script);
        } else if c.b("ontooff") && was_on && !on {
            fire_at(c, &source, "onOnToOff", std::slice::from_ref(&pv), &script);
        } else if c.b("valuechange") && (prev_val - value).abs() > f64::EPSILON {
            fire_at(
                c,
                &source,
                "onValueChange",
                &[pv.clone(), Value::Float(value), Value::Float(prev_val)],
                &script,
            );
        }
    }

    if prev != Some(value) {
        c.watched.values.insert("panel".into(), value);
    }

    if c.b("whileon") && on {
        fire_at(c, &source, "whileOn", &[Value::Str(key.clone())], &script);
    }
    if c.b("whileoff") && !on {
        fire_at(c, &source, "whileOff", &[Value::Str(key)], &script);
    }
    callback_dat_view(c, &source)
}
pub const TYPE_NAME: &str = "panelexecuteDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Panel Execute",
        &[],
        "Panel value callbacks.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
