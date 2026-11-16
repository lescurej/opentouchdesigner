use super::callbacks::fire_new_table_rows;
use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::DatData;

use super::constants::*;
use super::helpers::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    merge_params(
        params! {
            "active" => Param::bool(true).with_label("Active"),
            "port" => Param::str("/dev/ttyUSB0").with_label("Port"),
            "baudrate" => Param::int(9600).with_label("Baud Rate"),
            "maxlines" => Param::int(20).with_label("Maximum Lines").with_range(1.0, 1000.0),
        },
        network_callback_params(SERIAL_RECEIVE_CALLBACKS),
    )
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let path = c.path.to_string();
    if !c.b("active") {
        c.net.serial.remove(&path);
        c.watched.values.insert("_serial_rows".into(), 0.0);
        return crate::serial_io::empty_table();
    }
    let port_name = c.s("port");
    let baud = c.val("baudrate").as_i64().clamp(300, 921600) as u32;
    let key = crate::serial_io::link_key(&port_name, baud);
    let needs = c
        .net
        .serial
        .get(&path)
        .map(|s| s.key != key)
        .unwrap_or(true);
    if needs {
        c.net.serial.remove(&path);
        match crate::serial_io::SerialLink::open(&port_name, baud) {
            Ok(s) => {
                c.net.serial.insert(path.clone(), s);
                c.watched.values.insert("_serial_rows".into(), 0.0);
            }
            Err(e) => {
                c.error = Some(e);
                return crate::serial_io::empty_table();
            }
        }
    }
    let maxlines = c.val("maxlines").as_i64().clamp(1, 1000) as usize;
    let batch = {
        let link = c.net.serial.get_mut(&path).expect("serial");
        otd_core::cook_drain(&link.inbox, &mut link.display, crate::serial_io::KEEP_CAP)
    };
    let table = {
        let link = c.net.serial.get(&path).expect("serial");
        crate::serial_io::build_table(&link.display, maxlines)
    };
    crate::serial_io::log_rows(c.net, c.eval.frame, &path, &batch);
    fire_new_table_rows(c, "_serial_rows", &table, "onReceive", |row, t| {
        vec![
            Value::Int(row as i64),
            Value::Str(t.cell(row, 0).to_string()),
        ]
    });
    table
}

pub const TYPE_NAME: &str = "serialDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Serial",
        &[],
        "Serial lines.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
