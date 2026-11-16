use std::net::SocketAddr;

use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::shared::udp::{fire_udp_new_rows, params_udp_common};
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    let mut p = params_udp_common();
    p.insert(
        "address".into(),
        Param::str("127.0.0.1").with_label("Network Address"),
    );
    p.insert(
        "port".into(),
        Param::int(7001).with_label("Port").with_range(1.0, 65535.0),
    );
    p.insert(
        "sendonchange".into(),
        Param::bool(false).with_label("Send Input On Change"),
    );
    p
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let host = c.s("address");
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let active = c.b("active");
    let bytes_col = c.b("bytes");
    let path = c.path.to_string();

    if !active {
        c.net.udp_out.remove(&path);
        c.watched.values.insert("_udp_rows".into(), 0.0);
        return crate::udp::empty_table(bytes_col);
    }

    let Ok(target) = format!("{}:{}", host.trim(), port).parse::<SocketAddr>() else {
        c.error = Some(format!("bad UDP address `{host}:{port}`"));
        return crate::udp::empty_table(bytes_col);
    };

    let target_key = format!("{target}\0{local}");
    let needs_open = c
        .net
        .udp_out
        .get(&path)
        .map(|u| format!("{}\0{}", u.target, u.listener.local_address) != target_key)
        .unwrap_or(true);
    if needs_open {
        match crate::udp::UdpOutConnection::open(target, &local) {
            Ok(u) => {
                c.watched.values.insert("_udp_rows".into(), 0.0);
                c.net.udp_out.insert(path.clone(), u);
            }
            Err(e) => {
                c.error = Some(e);
                return crate::udp::empty_table(bytes_col);
            }
        }
    }

    if c.b("sendonchange") {
        let text = c.input(0).as_text();
        if let Some(out) = c.net.udp_out.get_mut(&path) {
            if out.last_sent.as_deref() != Some(text.as_str()) && !text.is_empty() {
                if let Err(e) = out.listener.send_to(text.as_bytes(), out.target) {
                    c.error = Some(e);
                } else {
                    out.last_sent = Some(text);
                }
            }
        }
    }

    let Some(out) = c.net.udp_out.get(&path) else {
        return crate::udp::empty_table(bytes_col);
    };
    let maxlines = crate::udp::maxlines_from_node(c.node);
    let format = c.menu("format");
    let inbox = out
        .listener
        .inbox
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();
    let table = crate::udp::build_table(&inbox, maxlines, format, bytes_col);
    fire_udp_new_rows(c, &table, bytes_col);
    table
}

pub const TYPE_NAME: &str = "udpoutDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "UDP Out",
        &["in"],
        "UDP send/receive table.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
