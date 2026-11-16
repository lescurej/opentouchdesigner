use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "address" => Param::str("127.0.0.1").with_label("Network Address"),
        "port" => Param::int(7001).with_label("Port").with_range(1.0, 65535.0),
        "oscaddress" => Param::str("/test").with_label("OSC Address"),
        "value" => Param::float(0.0).with_label("Float Value"),
        "sendonchange" => Param::bool(false).with_label("Send Input On Change"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    if !c.b("active") {
        return c.input(0);
    }
    let host = c.s("address");
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let addr = c.s("oscaddress");
    let value = c.val("value").as_f64() as f32;
    let send_text = c.input(0).as_text();
    let payload_key = if c.b("sendonchange") && !send_text.is_empty() {
        send_text.clone()
    } else {
        format!("{addr}:{value}")
    };
    let prev = c
        .watched
        .values
        .get("_osc_sent")
        .map(|v| v.to_string())
        .unwrap_or_default();
    if payload_key != prev {
        let result = if c.b("sendonchange") && !send_text.is_empty() {
            crate::osc::send_message(&host, port, send_text.trim(), 0.0)
        } else {
            crate::osc::send_message(&host, port, &addr, value)
        };
        if let Err(e) = result {
            c.error = Some(e);
        } else {
            c.watched
                .values
                .insert("_osc_sent".into(), payload_key.len() as f64);
            c.net
                .monitor
                .push(c.eval.frame, c.path, "osc-out", &format!("{addr} {value}"));
        }
    }
    c.input(0)
}
pub const TYPE_NAME: &str = "oscoutDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "OSC Out",
        &["in"],
        "OSC send.",
        params,
        cook,
    )
}
