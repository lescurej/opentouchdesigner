use super::super::*;

use crate::ops::ChopCtx;
pub(crate) struct OscOut {
    socket: UdpSocket,
    target: SocketAddr,
    local: String,
    sent: HashMap<String, f32>,
}

fn osc_address_from_channel(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        return "/".into();
    }
    if name.starts_with('/') {
        name.to_string()
    } else {
        format!("/{}", name.replace('_', "/"))
    }
}

fn osc_arg(value: f32, numericformat: usize) -> rosc::OscType {
    match numericformat {
        0 => rosc::OscType::Int(value.round() as i32),
        2 => rosc::OscType::Double(value as f64),
        _ => rosc::OscType::Float(value),
    }
}

fn osc_out_socket(local: &str) -> Result<UdpSocket, String> {
    let addr = if local.trim().is_empty() {
        SocketAddr::from(([0, 0, 0, 0], 0))
    } else {
        format!("{}:0", local.trim())
            .parse()
            .map_err(|_| format!("bad local address `{local}`"))?
    };
    UdpSocket::bind(addr).map_err(|e| format!("OSC out bind: {e}"))
}

pub(crate) fn params_osc_out() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m.insert(
        "protocol".into(),
        Param::menu("msging", &["msging", "multicastmsging", "reliablemsging"])
            .with_label("Protocol"),
    );
    m.insert(
        "netaddress".into(),
        Param::str("127.0.0.1").with_label("Network Address"),
    );
    m.insert(
        "port".into(),
        Param::int(7001)
            .with_label("Network Port")
            .with_range(1.0, 65535.0),
    );
    m.insert(
        "localaddress".into(),
        Param::str("").with_label("Local Address"),
    );
    m.insert(
        "maxsize".into(),
        Param::int(50)
            .with_label("Max Queue Size")
            .with_range(1.0, 10000.0),
    );
    m.insert(
        "maxsizeunit".into(),
        Param::menu("samples", &["samples", "frames", "seconds"]).with_label("Max Queue Size Unit"),
    );
    m.insert(
        "cookalways".into(),
        Param::bool(false).with_label("Cook Every Frame"),
    );
    m.insert(
        "numericformat".into(),
        Param::menu("float", &["int", "float", "double"]).with_label("Numeric Format"),
    );
    m.insert(
        "format".into(),
        Param::menu(
            "sample",
            &["sample", "timeslice", "transpose", "transposename"],
        )
        .with_label("Data Format"),
    );
    m.insert(
        "maxbytes".into(),
        Param::int(8192)
            .with_label("Max Message Bytes")
            .with_range(64.0, 65536.0),
    );
    m.insert(
        "sendevents".into(),
        Param::bool(true).with_label("Send Events Every Cook"),
    );
    m
}

fn osc_out_target(host: &str, port: u16, protocol: usize, net: &str) -> Result<SocketAddr, String> {
    if protocol == 1 {
        let group: Ipv4Addr = net
            .trim()
            .parse()
            .map_err(|_| format!("bad multicast address `{net}`"))?;
        return Ok(SocketAddr::from((group, port)));
    }
    let host = host.trim();
    let host = if host.eq_ignore_ascii_case("localhost") {
        "127.0.0.1"
    } else {
        host
    };
    format!("{host}:{port}")
        .parse()
        .map_err(|_| format!("bad OSC address `{host}:{port}`"))
}

fn osc_send(
    out: &mut OscOut,
    addr: &str,
    args: Vec<rosc::OscType>,
    sendevents: bool,
    sent_key: &str,
    fingerprint: f32,
) {
    if !sendevents {
        if out.sent.get(sent_key).copied() == Some(fingerprint) {
            return;
        }
        out.sent.insert(sent_key.to_string(), fingerprint);
    }
    let message = rosc::OscPacket::Message(rosc::OscMessage {
        addr: addr.to_string(),
        args,
    });
    if let Ok(bytes) = rosc::encoder::encode(&message) {
        let _ = out.socket.send_to(&bytes, out.target);
    }
}

pub(crate) fn cook_osc_out(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let path = c.path.to_string();

    if !c.b("active") || input.num_channels() == 0 {
        c.io.osc_out.remove(&path);
        return input;
    }

    let protocol = c.menu("protocol");
    if protocol == 2 {
        c.io.note(
            &path,
            "reliablemsging (UDT) is not supported; use msging or multicastmsging",
        );
        return input;
    }

    let net = c.s("netaddress");
    let port = c.i("port").clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let sendevents = c.b("sendevents");
    let numericformat = c.menu("numericformat");
    let format = c.menu("format");

    let target = match osc_out_target(&net, port, protocol, &net) {
        Ok(t) => t,
        Err(e) => {
            c.io.note(&path, e);
            return input;
        }
    };

    let needs_open =
        c.io.osc_out
            .get(&path)
            .map(|o| o.target != target || o.local != local)
            .unwrap_or(true);
    if needs_open {
        match osc_out_socket(&local) {
            Ok(socket) => {
                c.io.clear_note(&path);
                c.io.osc_out.insert(
                    path.clone(),
                    OscOut {
                        socket,
                        target,
                        local: local.clone(),
                        sent: HashMap::new(),
                    },
                );
            }
            Err(e) => c.io.note(&path, e),
        }
    }

    let Some(out) = c.io.osc_out.get_mut(&path) else {
        return input;
    };

    match format {
        1 => {
            for ch in &input.channels {
                let addr = osc_address_from_channel(&ch.name);
                let args: Vec<rosc::OscType> = ch
                    .samples
                    .iter()
                    .map(|v| osc_arg(*v, numericformat))
                    .collect();
                let fp = ch.last();
                osc_send(out, &addr, args, sendevents, &ch.name, fp);
            }
        }
        2 => {
            let args: Vec<rosc::OscType> = input
                .channels
                .iter()
                .map(|ch| osc_arg(ch.last(), numericformat))
                .collect();
            let fp = args.len() as f32;
            osc_send(out, "/", args, sendevents, "__transpose__", fp);
        }
        3 => {
            use std::collections::BTreeMap;
            let mut groups: BTreeMap<String, Vec<f32>> = BTreeMap::new();
            for ch in &input.channels {
                let root = ch
                    .name
                    .split('/')
                    .next()
                    .unwrap_or(ch.name.as_str())
                    .to_string();
                groups.entry(root).or_default().push(ch.last());
            }
            for (root, values) in groups {
                let addr = osc_address_from_channel(&root);
                let args: Vec<rosc::OscType> =
                    values.iter().map(|v| osc_arg(*v, numericformat)).collect();
                let fp = values.last().copied().unwrap_or(0.0);
                osc_send(out, &addr, args, sendevents, &root, fp);
            }
        }
        _ => {
            for ch in &input.channels {
                let addr = osc_address_from_channel(&ch.name);
                let value = ch.last();
                let args = vec![osc_arg(value, numericformat)];
                osc_send(out, &addr, args, sendevents, &ch.name, value);
            }
        }
    }

    input
}
