use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::DatData;

use super::callbacks::{callback_dat_view, fire};
use super::constants::*;
use super::helpers::selected;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "chop" => Param::str("").with_label("Watch CHOP").as_path_ref(),
        "channels" => Param::str("*").with_label("Channels"),
        "threshold" => Param::float(0.5).with_label("On Threshold").with_range(-10.0, 10.0),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(CHOP_EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    let path = c.s("chop");
    let Some(net) = c.eval.channels else {
        return callback_dat_view(c, &source);
    };
    let names = net.channel_names(path.trim());
    let threshold = c
        .node
        .param("threshold")
        .map(|p| p.eval(c.eval).as_f32())
        .unwrap_or(0.5);
    let pattern = c.s("channels");

    // Collected first, because firing a callback borrows `c` mutably and the
    // network borrow has to be finished by then.
    let mut events: Vec<(String, f64, Option<f64>)> = Vec::new();
    for name in selected(&pattern, &names) {
        let Some(value) = net.channel(path.trim(), name) else {
            continue;
        };
        let value = value as f64;
        let prev = c.watched.values.get(name).copied();
        if prev != Some(value) {
            events.push((name.clone(), value, prev));
        }
    }
    for (name, value, _) in &events {
        c.watched.values.insert(name.clone(), *value);
    }

    for (name, value, prev) in events {
        let args = [
            Value::Str(name.clone()),
            Value::Float(0.0),
            Value::Float(value),
            Value::Float(prev.unwrap_or(value)),
        ];
        fire(c, &source, "onValueChange", &args);
        // The edge callbacks are the ones worth having: "the beat landed" is
        // a different question from "the number moved".
        let Some(prev) = prev else { continue };
        let t = threshold as f64;
        if prev < t && value >= t {
            fire(
                c,
                &source,
                "onOffToOn",
                &[
                    Value::Str(name.clone()),
                    Value::Float(0.0),
                    Value::Float(value),
                    Value::Float(prev),
                ],
            );
        } else if prev >= t && value < t {
            fire(
                c,
                &source,
                "onOnToOff",
                &[
                    Value::Str(name),
                    Value::Float(0.0),
                    Value::Float(value),
                    Value::Float(prev),
                ],
            );
        }
    }
    callback_dat_view(c, &source)
}
pub const TYPE_NAME: &str = "chopexecuteDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "CHOP Execute",
        &[],
        "CHOP change callbacks.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
