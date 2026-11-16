use super::callbacks::{fire_dat_callback, fire_new_table_rows};
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
            "mode" => Param::menu("client", &["client", "server"]).with_label("Mode"),
            "address" => Param::str("127.0.0.1").with_label("Network Address"),
            "port" => Param::int(9000).with_label("Port").with_range(1.0, 65535.0),
            "localaddress" => Param::str("").with_label("Local Address"),
            "maxlines" => Param::int(20).with_label("Maximum Lines").with_range(1.0, 1000.0),
            "sendonchange" => Param::bool(false).with_label("Send Input On Change"),
        },
        network_callback_params(TCPIP_RECEIVE_CALLBACKS),
    )
}

fn tcp_set_connected(c: &mut DatCtx, connected: bool) {
    let was = c.watched.values.get("_tcp_open").copied().unwrap_or(0.0) > 0.5;
    if connected && !was {
        fire_dat_callback(c, "onConnect", &[]);
        c.watched.values.insert("_tcp_open".into(), 1.0);
    } else if !connected && was {
        fire_dat_callback(c, "onDisconnect", &[]);
        c.watched.values.insert("_tcp_open".into(), 0.0);
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let path = c.path.to_string();
    if !c.b("active") {
        tcp_set_connected(c, false);
        c.net.tcp_client.remove(&path);
        c.net.tcp_server.remove(&path);
        c.watched.values.insert("_tcp_rows".into(), 0.0);
        return crate::tcp::empty_table();
    }
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let host = c.s("address");
    let local = c.s("localaddress");
    let maxlines = c.val("maxlines").as_i64().clamp(1, 1000) as usize;
    if c.menu("mode") == 1 {
        let key = crate::tcp::server_key(port, &local);
        let needs = c
            .net
            .tcp_server
            .get(&path)
            .map(|s| s.key != key)
            .unwrap_or(true);
        if needs {
            c.net.tcp_server.remove(&path);
            match crate::tcp::TcpServer::bind(port, &local) {
                Ok(s) => {
                    c.net.tcp_server.insert(path.clone(), s);
                    c.watched.values.insert("_tcp_rows".into(), 0.0);
                    tcp_set_connected(c, true);
                }
                Err(e) => {
                    c.error = Some(e);
                    tcp_set_connected(c, false);
                    return crate::tcp::empty_table();
                }
            }
        }
        let batch = {
            let server = c.net.tcp_server.get_mut(&path).expect("server");
            otd_core::cook_drain(&server.inbox, &mut server.display, crate::tcp::KEEP_CAP)
        };
        let table = {
            let server = c.net.tcp_server.get(&path).expect("server");
            crate::tcp::build_table(&server.display, maxlines)
        };
        crate::tcp::log_rows(c.net, c.eval.frame, &path, &batch);
        fire_new_table_rows(c, "_tcp_rows", &table, "onReceive", |row, t| {
            vec![
                Value::Int(row as i64),
                Value::Str(t.cell(row, 0).to_string()),
                Value::Str(t.cell(row, 1).to_string()),
            ]
        });
        return table;
    }
    let key = crate::tcp::client_key(&host, port);
    let needs = c
        .net
        .tcp_client
        .get(&path)
        .map(|s| s.key != key)
        .unwrap_or(true);
    if needs {
        tcp_set_connected(c, false);
        c.net.tcp_client.remove(&path);
        match crate::tcp::TcpClient::connect(&host, port) {
            Ok(s) => {
                c.net.tcp_client.insert(path.clone(), s);
                c.watched.values.insert("_tcp_rows".into(), 0.0);
                tcp_set_connected(c, true);
            }
            Err(e) => {
                c.error = Some(e);
                tcp_set_connected(c, false);
                return crate::tcp::empty_table();
            }
        }
    }
    if c.b("sendonchange") {
        let text = c.input(0).as_text();
        if !text.is_empty() {
            if let Err(e) = crate::tcp::send_line(&host, port, &text) {
                c.error = Some(e);
            } else {
                c.net.monitor.push(c.eval.frame, &path, "tcp-out", &text);
            }
        }
    }
    let batch = {
        let client = c.net.tcp_client.get_mut(&path).expect("client");
        otd_core::cook_drain(&client.inbox, &mut client.display, crate::tcp::KEEP_CAP)
    };
    let table = {
        let client = c.net.tcp_client.get(&path).expect("client");
        crate::tcp::build_table(&client.display, maxlines)
    };
    crate::tcp::log_rows(c.net, c.eval.frame, &path, &batch);
    fire_new_table_rows(c, "_tcp_rows", &table, "onReceive", |row, t| {
        vec![
            Value::Int(row as i64),
            Value::Str(t.cell(row, 0).to_string()),
            Value::Str(t.cell(row, 1).to_string()),
        ]
    });
    table
}
pub const TYPE_NAME: &str = "tcpipDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "TCP/IP",
        &["in"],
        "TCP client/server.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
