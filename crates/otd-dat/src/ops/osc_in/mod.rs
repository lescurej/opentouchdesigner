use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "protocol" => Param::menu("msging", &["msging", "multicastmsging"]).with_label("Protocol"),
        "netaddress" => Param::str("239.255.0.1").with_label("Network Address"),
        "port" => Param::int(7000).with_label("Port").with_range(1.0, 65535.0),
        "localaddress" => Param::str("").with_label("Local Address"),
        "maxlines" => Param::int(20).with_label("Maximum Lines").with_range(1.0, 1000.0),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let path = c.path.to_string();
    let active = c.b("active");
    if !active {
        c.net.osc_in.remove(&path);
        return crate::osc::empty_table();
    }
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let multicast = c.menu("protocol") == 1;
    let net = c.s("netaddress");
    let key = crate::osc::listener_key(port, &local, multicast, &net);
    let needs_open = c
        .net
        .osc_in
        .get(&path)
        .map(|o| o.key != key)
        .unwrap_or(true);
    if needs_open {
        c.net.osc_in.remove(&path);
        match crate::osc::OscListener::open(port, &local, multicast, &net) {
            Ok(o) => {
                c.net.osc_in.insert(path.clone(), o);
            }
            Err(e) => {
                c.error = Some(e);
                return crate::osc::empty_table();
            }
        }
    }
    let maxlines = c.val("maxlines").as_i64().clamp(1, 1000) as usize;
    let batch = {
        let listener = c.net.osc_in.get_mut(&path).expect("osc");
        otd_core::cook_drain(&listener.inbox, &mut listener.display, crate::osc::KEEP_CAP)
    };
    let table = {
        let listener = c.net.osc_in.get(&path).expect("osc");
        crate::osc::build_table(&listener.display, maxlines)
    };
    crate::osc::log_rows(c.net, c.eval.frame, &path, &batch);
    table
}
pub const TYPE_NAME: &str = "oscinDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "OSC In",
        &[],
        "OSC receive table.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
