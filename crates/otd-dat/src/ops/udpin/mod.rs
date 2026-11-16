
use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::shared::udp::{fire_udp_new_rows, params_udp_common, udp_listener_key};
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    let mut p = params_udp_common();
    p.insert(
        "port".into(),
        Param::int(7000).with_label("Port").with_range(1.0, 65535.0),
    );
    p
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let path = c.path.to_string();
    let bytes_col = c.b("bytes");
    let active = c.b("active");

    if !active {
        c.net.udp_in.remove(&path);
        c.watched.values.insert("_udp_rows".into(), 0.0);
        return crate::udp::empty_table(bytes_col);
    }

    let key = udp_listener_key(port, &local);
    let needs_open = c
        .net
        .udp_in
        .get(&path)
        .map(|u| udp_listener_key(u.port, &u.local_address) != key)
        .unwrap_or(true);
    if needs_open {
        c.net.udp_in.remove(&path);
        match crate::udp::UdpListener::bind(port, &local) {
            Ok(u) => {
                c.watched.values.insert("_udp_rows".into(), 0.0);
                c.net.udp_in.insert(path.clone(), u);
            }
            Err(e) => {
                c.error = Some(e);
                return crate::udp::empty_table(bytes_col);
            }
        }
    }

    if !c.net.udp_in.contains_key(&path) {
        return crate::udp::empty_table(bytes_col);
    };
    let maxlines = crate::udp::maxlines_from_node(c.node);
    let format = c.menu("format");
    let batch = {
        let listener = c.net.udp_in.get_mut(&path).expect("udp");
        otd_core::cook_drain(&listener.inbox, &mut listener.display, crate::udp::KEEP_CAP)
    };
    let _ = batch;
    let table = {
        let listener = c.net.udp_in.get(&path).expect("udp");
        crate::udp::build_table(&listener.display, maxlines, format, bytes_col)
    };
    fire_udp_new_rows(c, &table, bytes_col);
    table
}
pub const TYPE_NAME: &str = "udpinDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "UDP In",
        &[],
        "UDP listen table.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
