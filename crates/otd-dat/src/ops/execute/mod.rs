use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::DatData;

use super::callbacks::{callback_dat_view, fire};
use super::constants::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    if !c.watched.started {
        c.watched.started = true;
        fire(c, &source, "onStart", &[]);
    }
    let frame = Value::Int(c.eval.frame);
    fire(c, &source, "onFrameStart", std::slice::from_ref(&frame));
    // Both edges of the frame fire here, one cook apart in the same cook.
    // A genuine end-of-frame hook would have to run after every other node,
    // and the cook is demand-driven — there is no such moment to hang it on
    // that is not simply "later", so saying so beats pretending.
    fire(c, &source, "onFrameEnd", std::slice::from_ref(&frame));
    callback_dat_view(c, &source)
}
pub const TYPE_NAME: &str = "executeDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Execute",
        &[],
        "Frame Python callbacks.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
