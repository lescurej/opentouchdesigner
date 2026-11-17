use super::super::*;

use crate::ops::ChopCtx;
pub(crate) struct OscIn {
    values: Arc<Mutex<Vec<(String, f32)>>>,
    stop: Arc<AtomicBool>,
    freeze: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    key: String,
}

impl Drop for OscIn {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

fn osc_listener_key(port: u16, local: &str, multicast: bool, net: &str) -> String {
    format!("{}\0{port}\0{multicast}\0{net}", local.trim())
}

fn bind_udp(port: u16, local: &str) -> Result<UdpSocket, String> {
    let addr = if local.trim().is_empty() {
        SocketAddr::from(([0, 0, 0, 0], port))
    } else {
        format!("{}:{port}", local.trim())
            .parse()
            .map_err(|_| format!("bad local address `{local}:{port}`"))?
    };
    let socket = UdpSocket::bind(addr).map_err(|e| format!("OSC bind {addr}: {e}"))?;
    socket
        .set_read_timeout(Some(std::time::Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    Ok(socket)
}

impl OscIn {
    fn open(port: u16, local: &str, multicast: bool, net: &str) -> Result<OscIn, String> {
        let socket = bind_udp(port, local)?;
        if multicast {
            let group: Ipv4Addr = net
                .trim()
                .parse()
                .map_err(|_| format!("bad multicast address `{net}`"))?;
            let iface = if local.trim().is_empty() {
                Ipv4Addr::UNSPECIFIED
            } else {
                local
                    .trim()
                    .parse()
                    .map_err(|_| format!("bad local address `{local}`"))?
            };
            socket
                .join_multicast_v4(&group, &iface)
                .map_err(|e| format!("OSC multicast join {group}: {e}"))?;
        }

        let values: Arc<Mutex<Vec<(String, f32)>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = values.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let freeze = Arc::new(AtomicBool::new(false));
        let freeze_flag = freeze.clone();
        let key = osc_listener_key(port, local, multicast, net);

        let thread = std::thread::Builder::new()
            .name(format!("otd-osc-in-{port}"))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok((len, _)) = socket.recv_from(&mut buf) else {
                        continue;
                    };
                    if freeze_flag.load(Ordering::Relaxed) {
                        continue;
                    }
                    let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..len]) else {
                        continue;
                    };
                    let Ok(mut values) = sink.lock() else { break };
                    collect_osc(&packet, &mut values);
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(OscIn {
            values,
            stop,
            freeze,
            _thread: Some(thread),
            key,
        })
    }
}

/// Flatten a packet into `(channel name, value)` pairs. Only the leading
/// `/` is dropped; the rest of the address, including `/`, is the channel
/// name (`/test/1` → `test/1`). Several arguments become `path_1`, `path_2`, ...
/// `/_samplerate` is TouchDesigner sender metadata and is not a channel.
pub(crate) fn collect_osc(packet: &rosc::OscPacket, out: &mut Vec<(String, f32)>) {
    match packet {
        rosc::OscPacket::Bundle(b) => {
            for p in &b.content {
                collect_osc(p, out);
            }
        }
        rosc::OscPacket::Message(m) => {
            let base = m.addr.strip_prefix('/').unwrap_or(m.addr.as_str());
            if base == "_samplerate" {
                return;
            }
            for (i, arg) in m.args.iter().enumerate() {
                let value = match arg {
                    rosc::OscType::Float(f) => *f,
                    rosc::OscType::Double(d) => *d as f32,
                    rosc::OscType::Int(i) => *i as f32,
                    rosc::OscType::Long(l) => *l as f32,
                    rosc::OscType::Bool(b) => {
                        if *b {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    _ => continue,
                };
                let name = if m.args.len() == 1 {
                    base.to_string()
                } else {
                    format!("{base}_{}", i + 1)
                };
                match out.iter_mut().find(|(n, _)| *n == name) {
                    Some(slot) => slot.1 = value,
                    None => out.push((name, value)),
                }
            }
        }
    }
}

fn channel_passes_osc_scope(scope: &str, name: &str) -> bool {
    let scope = scope.trim();
    if scope.is_empty() {
        return true;
    }
    let mut include_any = false;
    let mut has_include_rule = false;
    for pat in scope.split_whitespace() {
        if let Some(exclude) = pat.strip_prefix('^') {
            if otd_core::pattern_matches_channel(exclude, name) {
                return false;
            }
        } else {
            has_include_rule = true;
            if otd_core::pattern_matches_channel(pat, name) {
                include_any = true;
            }
        }
    }
    !has_include_rule || include_any
}

pub(crate) fn params_osc_in() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m.insert(
        "protocol".into(),
        Param::menu("msging", &["msging", "multicastmsging"]).with_label("Protocol"),
    );
    m.insert(
        "netaddress".into(),
        Param::str("239.255.0.1").with_label("Network Address"),
    );
    m.insert(
        "port".into(),
        Param::int(7000)
            .with_label("Network Port")
            .with_range(1.0, 65535.0),
    );
    m.insert(
        "localaddress".into(),
        Param::str("").with_label("Local Address"),
    );
    m.insert(
        "oscaddressscope".into(),
        Param::str("").with_label("OSC Address Scope"),
    );
    m.insert(
        "useglobalrate".into(),
        Param::bool(true).with_label("Use Global Rate"),
    );
    m.insert(
        "samplerate".into(),
        Param::float(CONTROL_RATE)
            .with_label("Default Sample Rate")
            .with_range(1.0, 48000.0),
    );
    m.insert("reset".into(), Param::bool(false).with_label("Reset"));
    m
}

pub(crate) fn cook_osc_in(c: &mut ChopCtx) -> ChopData {
    let rate = if c.b("useglobalrate") {
        c.eval.fps
    } else {
        c.f("samplerate") as f64
    }
    .max(1.0);
    let n = slice_len(rate, c.time);
    let scope = c.s("oscaddressscope");
    let path = c.path.to_string();

    if !c.b("active") {
        c.io.osc_in.remove(&path);
        return ChopData::new(vec![Channel::constant("chan1", 0.0, n)], rate, true);
    }

    let port = c.i("port").clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let multicast = c.menu("protocol") == 1;
    let net = c.s("netaddress");
    let key = osc_listener_key(port, &local, multicast, &net);

    let needs_open = c.io.osc_in.get(&path).map(|o| o.key != key).unwrap_or(true);
    if needs_open {
        c.io.osc_in.remove(&path);
        match OscIn::open(port, &local, multicast, &net) {
            Ok(o) => {
                c.io.clear_note(&path);
                c.io.osc_in.insert(path.clone(), o);
            }
            Err(e) => c.io.note(&path, e),
        }
    }

    let reset = c.b("reset");
    let mut channels = Vec::new();
    if let Some(osc) = c.io.osc_in.get(&path) {
        osc.freeze.store(reset, Ordering::Relaxed);
        if reset {
            if let Ok(mut values) = osc.values.lock() {
                values.clear();
            }
        } else if let Ok(values) = osc.values.lock() {
            for (name, value) in values.iter() {
                if channel_passes_osc_scope(&scope, name) {
                    channels.push(Channel::constant(name.clone(), *value, n));
                }
            }
        }
    }
    if channels.is_empty() {
        channels.push(Channel::constant("chan1", 0.0, n));
    }
    ChopData::new(channels, rate, true)
}
