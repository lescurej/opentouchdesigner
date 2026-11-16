use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "ops" => Param::str("").with_label("Panel COMPs (paths)"),
    }
}

/// The values of some panel widgets, as channels.
///
/// A widget's state is an ordinary parameter, so Bind mode already reaches it
/// and this operator is a convenience rather than a mechanism. It earns its
/// place when there are eight faders: one node with eight channels beats eight
/// parameters each bound separately, and the result filters, lags and exports
/// like anything else.
///
/// Paths rather than a search: enumerating the graph is not something the
/// narrow view a CHOP gets should be widened to allow, and naming what you
/// want is a line of text.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let paths = c.s("ops");
    let channels: Vec<Channel> = paths
        .split_whitespace()
        .map(|path| {
            let value = c
                .eval
                .channels
                .and_then(|net| net.param_value(path, "value"))
                .map(|v| v.as_f32())
                .unwrap_or(0.0);
            // Named after the node, not the whole path, so `/panel/fader1`
            // reads as `fader1` in an expression.
            let name = path.rsplit('/').next().unwrap_or(path);
            Channel::constant(crate::ops::shared::sanitise::sanitise(name), value, 1)
        })
        .collect();
    ChopData::new(channels, CONTROL_RATE, false)
}



pub const TYPE_NAME: &str = "panelCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Panel",
                &[],
                "Panel widget values as channels.",
                params,
                cook,
            )
}
