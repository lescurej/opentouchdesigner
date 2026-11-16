use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::DatData;

use super::constants::*;
use super::callbacks::{fire_new_table_rows, ws_set_open};
use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    merge_params(
        params! {
            "active" => Param::bool(true).with_label("Active"),
            "url" => Param::str("ws://127.0.0.1:9980/").with_label("URL"),
            "maxlines" => Param::int(20).with_label("Maximum Lines").with_range(1.0, 1000.0),
        },
        network_callback_params(WEBSOCKET_CALLBACKS),
    )
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let path = c.path.to_string();
    if !c.b("active") {
        ws_set_open(c, false);
        c.net.ws_client.remove(&path);
        c.watched.values.insert("_ws_rows".into(), 0.0);
        return crate::ws::empty_table();
    }
    let url = c.s("url");
    let needs = c
        .net
        .ws_client
        .get(&path)
        .map(|w| w.url != url)
        .unwrap_or(true);
    if needs {
        ws_set_open(c, false);
        c.net.ws_client.remove(&path);
        match crate::ws::WsClient::connect(&url) {
            Ok(w) => {
                c.net.ws_client.insert(path.clone(), w);
                c.watched.values.insert("_ws_rows".into(), 0.0);
            }
            Err(e) => {
                c.error = Some(e);
                ws_set_open(c, false);
                return crate::ws::empty_table();
            }
        }
    }
    let connected = c
        .net
        .ws_client
        .get(&path)
        .map(|s| s.connected.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false);
    ws_set_open(c, connected);
    let maxlines = c.val("maxlines").as_i64().clamp(1, 1000) as usize;
    let batch = {
        let client = c.net.ws_client.get_mut(&path).expect("ws");
        otd_core::cook_drain(&client.inbox, &mut client.display, crate::ws::KEEP_CAP)
    };
    let table = {
        let client = c.net.ws_client.get(&path).expect("ws");
        crate::ws::build_table(&client.display, maxlines)
    };
    crate::ws::log_rows(c.net, c.eval.frame, &path, &batch);
    fire_new_table_rows(c, "_ws_rows", &table, "onReceiveText", |row, t| {
        vec![
            Value::Int(row as i64),
            Value::Str(t.cell(row, 0).to_string()),
        ]
    });
    table
}
pub const TYPE_NAME: &str = "websocketDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "WebSocket",
        &[],
        "WebSocket client.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
