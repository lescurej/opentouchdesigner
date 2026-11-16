use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::callbacks::{callback_dat_view, dat_script_path};
use super::constants::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "port" => Param::int(9980).with_label("Port").with_range(1.0, 65535.0),
        "localaddress" => Param::str("").with_label("Local Address"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(WEBSERVER_CALLBACKS).with_label("Source").into_script(),
    }
}

fn webserver_status_table(port: u16, address: &str, requests: u64) -> DatData {
    DatData::table(vec![
        vec!["name".into(), "value".into()],
        vec!["port".into(), port.to_string()],
        vec!["address".into(), address.to_string()],
        vec!["requests".into(), requests.to_string()],
    ])
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    let port_raw = c.val("port").as_i64();
    let port = if port_raw == 0 {
        0u16
    } else {
        port_raw.clamp(1, 65535) as u16
    };
    let bind = c.s("localaddress");
    let path = c.path.to_string();
    let dat_name = c.node.name.clone();
    let active = c.b("active");

    if !active {
        c.net.webserver.remove(&path);
        if c.watched.started {
            c.watched.started = false;
            if let Some(host) = c.scripts {
                let script_path = dat_script_path(c);
                let _ = host.webserver_event(
                    &source,
                    "onServerStop",
                    &c.node.name,
                    c.eval,
                    &script_path,
                );
            }
        }
        c.watched.values.insert("_requests".into(), 0.0);
        return callback_dat_view(c, &source);
    }

    let key = crate::webserver::server_key(port, &bind);
    let needs_open = c
        .net
        .webserver
        .get(&path)
        .map(|s| crate::webserver::server_key(s.listen_port, &s.bind) != key)
        .unwrap_or(true);
    if needs_open {
        c.net.webserver.remove(&path);
        match crate::webserver::WebServer::start(port, &bind) {
            Ok(srv) => {
                let bound_port = srv.port;
                let local = srv.local_address.clone();
                c.net.webserver.insert(path.clone(), srv);
                c.watched.values.insert("_requests".into(), 0.0);
                if !c.watched.started {
                    c.watched.started = true;
                    if let Some(host) = c.scripts {
                        let script_path = dat_script_path(c);
                        let _ = host.webserver_event(
                            &source,
                            "onServerStart",
                            &dat_name,
                            c.eval,
                            &script_path,
                        );
                    }
                }
                let req_count = c.watched.values.get("_requests").copied().unwrap_or(0.0) as u64;
                return webserver_status_table(bound_port, &local, req_count);
            }
            Err(e) => {
                c.error = Some(e);
                return callback_dat_view(c, &source);
            }
        }
    }

    let (bound_port, local, jobs, ws_events) = {
        let Some(server) = c.net.webserver.get(&path) else {
            return callback_dat_view(c, &source);
        };
        (
            server.port,
            server.local_address.clone(),
            server.drain_pending(),
            server.drain_ws_events(),
        )
    };

    let Some(host) = c.scripts else {
        for job in jobs {
            let _ = job.respond.send(crate::webserver::HttpReply {
                status: 503,
                reason: "Service Unavailable".into(),
                body: "Python is not available".into(),
                content_type: Some("text/plain".into()),
            });
        }
        c.error = Some("no Python interpreter in this build".into());
        let req_count = c.watched.values.get("_requests").copied().unwrap_or(0.0) as u64;
        return webserver_status_table(bound_port, &local, req_count);
    };

    for job in jobs {
        let script_path = dat_script_path(c);
        match host.webserver_http(&source, &dat_name, &job, c.eval, &script_path) {
            Ok(reply) => {
                let _ = job.respond.send(reply);
                let n = c.watched.values.get("_requests").copied().unwrap_or(0.0) + 1.0;
                c.watched.values.insert("_requests".into(), n);
            }
            Err(e) => {
                let _ = job.respond.send(crate::webserver::HttpReply {
                    status: 500,
                    reason: "Internal Server Error".into(),
                    body: e.clone(),
                    content_type: Some("text/plain".into()),
                });
                if c.error.is_none() {
                    c.error = Some(e);
                }
            }
        }
    }

    let script_path = dat_script_path(c);
    for ev in ws_events {
        let func = match &ev {
            crate::webserver::WebSocketEvent::Open { .. } => "onWebSocketOpen",
            crate::webserver::WebSocketEvent::Close { .. } => "onWebSocketClose",
            crate::webserver::WebSocketEvent::Text { .. } => "onWebSocketReceiveText",
            crate::webserver::WebSocketEvent::Binary { .. } => "onWebSocketReceiveBinary",
            crate::webserver::WebSocketEvent::Ping { .. } => "onWebSocketReceivePing",
            crate::webserver::WebSocketEvent::Pong { .. } => "onWebSocketReceivePong",
        };
        let client = match &ev {
            crate::webserver::WebSocketEvent::Open { client, .. }
            | crate::webserver::WebSocketEvent::Close { client }
            | crate::webserver::WebSocketEvent::Text { client, .. }
            | crate::webserver::WebSocketEvent::Binary { client, .. }
            | crate::webserver::WebSocketEvent::Ping { client, .. }
            | crate::webserver::WebSocketEvent::Pong { client, .. } => client.as_str(),
        };
        let uri = match &ev {
            crate::webserver::WebSocketEvent::Open { uri, .. } => Some(uri.as_str()),
            _ => None,
        };
        let text = match &ev {
            crate::webserver::WebSocketEvent::Text { data, .. } => Some(data.as_str()),
            _ => None,
        };
        let binary = match &ev {
            crate::webserver::WebSocketEvent::Binary { data, .. }
            | crate::webserver::WebSocketEvent::Ping { data, .. }
            | crate::webserver::WebSocketEvent::Pong { data, .. } => Some(data.as_slice()),
            _ => None,
        };
        if let Err(e) = host.webserver_ws(
            &source,
            func,
            &dat_name,
            client,
            uri,
            text,
            binary,
            c.eval,
            &script_path,
        ) {
            if c.error.is_none() {
                c.error = Some(e);
            }
        }
    }

    if let Some(server) = c.net.webserver.get(&path) {
        let sends = otd_core::take_queued_ws_sends_for(&path);
        server.apply_ws_sends(&sends);
    }

    let req_count = c.watched.values.get("_requests").copied().unwrap_or(0.0) as u64;
    webserver_status_table(bound_port, &local, req_count)
}
pub const TYPE_NAME: &str = "webserverDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Web Server",
        &[],
        "HTTP server DAT.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
