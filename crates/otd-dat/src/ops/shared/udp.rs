
use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::DatData;

use super::super::callbacks::fire;
use super::super::constants::UDP_RECEIVE_CALLBACKS;
use super::super::DatCtx;

pub(crate) fn params_udp_common() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "localaddress" => Param::str("").with_label("Local Address"),
        "format" => Param::menu("permessage", &["permessage", "perline", "perbyte"]).with_label("Row Format"),
        "maxlines" => Param::int(20).with_label("Maximum Lines").with_range(1.0, 1000.0),
        "bytes" => Param::bool(false).with_label("Bytes Column"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(UDP_RECEIVE_CALLBACKS).with_label("Source").into_script(),
    }
}

pub(crate) fn udp_listener_key(port: u16, local: &str) -> String {
    format!("{local}\0{port}")
}

pub(crate) fn fire_udp_new_rows(c: &mut DatCtx, table: &DatData, bytes_col: bool) {
    let source = c.callback_script();
    if source.trim().is_empty() {
        return;
    }
    let data_rows = table.num_rows().saturating_sub(1);
    let prev = c.watched.values.get("_udp_rows").copied().unwrap_or(0.0) as usize;
    if data_rows < prev {
        c.watched
            .values
            .insert("_udp_rows".into(), data_rows as f64);
        return;
    }
    for row in (prev + 1)..=data_rows {
        let row_index = row;
        let message = table.cell(row, 2).to_string();
        let byte_str = if bytes_col {
            table.cell(row, 3).to_string()
        } else {
            String::new()
        };
        let peer = format!("{}:{}", table.cell(row, 0), table.cell(row, 1));
        fire(
            c,
            &source,
            "onReceive",
            &[
                Value::Int(row_index as i64),
                Value::Str(message),
                Value::Str(byte_str),
                Value::Str(peer),
            ],
        );
    }
    c.watched
        .values
        .insert("_udp_rows".into(), data_rows as f64);
}