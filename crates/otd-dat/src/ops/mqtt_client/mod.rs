use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::constants::*;
use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    merge_params(
        params! {
            "broker" => Param::str("").with_label("Broker URL"),
        },
        network_callback_params(MQTT_CLIENT_CALLBACKS),
    )
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let _ = c.s("broker");
    c.error = Some("MQTT Client DAT is not available in this build yet.".into());
    DatData::table(vec![vec!["status".into(), "unavailable".into()]])
}
pub const TYPE_NAME: &str = "mqttclientDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "MQTT Client",
        &[],
        "MQTT (stub).",
        params,
        cook,
    )
}
