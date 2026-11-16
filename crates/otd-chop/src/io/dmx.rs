//! I/O submodule: dmx.

use crate::ops::ChopCtx;

use super::*;

// ------------------------------------------------------------- DMX out

pub(crate) struct DmxOut {
    socket: UdpSocket,
    target: SocketAddr,
    sequence: u8,
    cid: [u8; 16],
}

pub(crate) fn params_dmx_out() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "protocol".into(),
        Param::menu("artnet", &["artnet", "sacn"]).with_label("Protocol"),
    );
    m.insert(
        "address".into(),
        Param::str("").with_label("Address (blank = broadcast / multicast)"),
    );
    m.insert(
        "universe".into(),
        Param::int(0)
            .with_label("Universe")
            .with_range(0.0, 32767.0),
    );
    m.insert(
        "start".into(),
        Param::int(1)
            .with_label("Start Channel")
            .with_range(1.0, 512.0),
    );
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m
}

pub(crate) fn cook_dmx_out(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let get = |k: &str| c.node.param(k).map(|p| p.eval(c.eval));
    if !get("active").map(|v| v.as_bool()).unwrap_or(true) || input.num_channels() == 0 {
        return input;
    }
    let sacn = c
        .node
        .param("protocol")
        .map(|p| p.menu_index(&p.eval(c.eval).as_str()))
        .unwrap_or(0)
        == 1;
    let universe = get("universe")
        .map(|v| v.as_i64())
        .unwrap_or(0)
        .clamp(0, 32767) as u16;
    let start = get("start").map(|v| v.as_i64()).unwrap_or(1).clamp(1, 512) as usize;
    let host = get("address").map(|v| v.as_str()).unwrap_or_default();
    let path = c.path.to_string();

    // DMX is a state protocol: each frame carries the current level of every
    // slot, which is the last sample of each channel.
    let mut slots = vec![0u8; crate::dmx::UNIVERSE_SIZE];
    for (i, ch) in input.channels.iter().enumerate() {
        let slot = start - 1 + i;
        if slot < slots.len() {
            slots[slot] = crate::dmx::to_slot(ch.last());
        }
    }
    let used = (start - 1 + input.num_channels()).min(crate::dmx::UNIVERSE_SIZE);

    let target: SocketAddr = if host.trim().is_empty() {
        if sacn {
            SocketAddr::from((crate::dmx::sacn_multicast(universe), crate::dmx::SACN_PORT))
        } else {
            // Art-Net's convention is a directed broadcast.
            SocketAddr::from(([255, 255, 255, 255], crate::dmx::ARTNET_PORT))
        }
    } else {
        let port = if sacn {
            crate::dmx::SACN_PORT
        } else {
            crate::dmx::ARTNET_PORT
        };
        match format!("{}:{port}", host.trim()).parse() {
            Ok(a) => a,
            Err(_) => {
                c.io.note(&path, format!("bad DMX address `{host}`"));
                return input;
            }
        }
    };

    if !c.io.dmx.contains_key(&path) {
        match UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], 0))) {
            Ok(socket) => {
                // Art-Net expects broadcast; without this the send fails on
                // every platform that enforces it.
                let _ = socket.set_broadcast(true);
                let mut cid = [0u8; 16];
                for (i, b) in path.bytes().take(16).enumerate() {
                    cid[i] = b;
                }
                c.io.clear_note(&path);
                c.io.dmx.insert(
                    path.clone(),
                    DmxOut {
                        socket,
                        target,
                        sequence: 0,
                        cid,
                    },
                );
            }
            Err(e) => c.io.note(&path, e.to_string()),
        }
    }

    if let Some(out) = c.io.dmx.get_mut(&path) {
        out.target = target;
        out.sequence = out.sequence.wrapping_add(1);
        let packet = if sacn {
            crate::dmx::sacn_packet(
                universe,
                out.sequence,
                "OpenTouchDesigner",
                &out.cid,
                &slots[..used],
            )
        } else {
            crate::dmx::artnet_packet(universe, out.sequence, &slots[..used])
        };
        if let Err(e) = out.socket.send_to(&packet, out.target) {
            c.io.note(&path, format!("DMX send: {e}"));
        }
    }
    input
}

// -------------------------------------------------------------- DMX in

pub(crate) struct DmxIn {
    slots: Arc<Mutex<[u8; crate::dmx::UNIVERSE_SIZE]>>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    key: String,
}

impl Drop for DmxIn {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

fn dmx_listener_key(sacn: bool, universe: u16, local: &str) -> String {
    format!("{sacn}\0{universe}\0{}", local.trim())
}

fn bind_dmx_socket(port: u16, local: &str) -> Result<UdpSocket, String> {
    let addr = if local.trim().is_empty() {
        SocketAddr::from(([0, 0, 0, 0], port))
    } else {
        format!("{}:{port}", local.trim())
            .parse()
            .map_err(|_| format!("bad local address `{local}:{port}`"))?
    };
    let socket = UdpSocket::bind(addr).map_err(|e| format!("DMX bind {addr}: {e}"))?;
    socket
        .set_read_timeout(Some(std::time::Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    Ok(socket)
}

impl DmxIn {
    fn open(sacn: bool, universe: u16, local: &str) -> Result<DmxIn, String> {
        let port = if sacn {
            crate::dmx::SACN_PORT
        } else {
            crate::dmx::ARTNET_PORT
        };
        let socket = bind_dmx_socket(port, local)?;
        if sacn {
            let group = crate::dmx::sacn_multicast(universe);
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
                .map_err(|e| format!("DMX multicast join {group}: {e}"))?;
        }

        let slots: Arc<Mutex<[u8; crate::dmx::UNIVERSE_SIZE]>> =
            Arc::new(Mutex::new([0u8; crate::dmx::UNIVERSE_SIZE]));
        let sink = slots.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let key = dmx_listener_key(sacn, universe, local);
        let watch = universe;

        let thread = std::thread::Builder::new()
            .name(format!("otd-dmx-in-{port}"))
            .spawn(move || {
                let mut buf = [0u8; 1200];
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok((len, _)) = socket.recv_from(&mut buf) else {
                        continue;
                    };
                    let slice = &buf[..len];
                    let parsed = if sacn {
                        crate::dmx::parse_sacn_dmx(slice)
                    } else {
                        crate::dmx::parse_artnet_dmx(slice)
                    };
                    let Some((u, data)) = parsed else {
                        continue;
                    };
                    if u != watch {
                        continue;
                    }
                    let Ok(mut slots) = sink.lock() else {
                        break;
                    };
                    let n = data.len().min(slots.len());
                    slots[..n].copy_from_slice(&data[..n]);
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(DmxIn {
            slots,
            stop,
            _thread: Some(thread),
            key,
        })
    }
}

pub(crate) fn params_dmx_in() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "protocol".into(),
        Param::menu("artnet", &["artnet", "sacn"]).with_label("Protocol"),
    );
    m.insert(
        "address".into(),
        Param::str("").with_label("Local Address (blank = all interfaces)"),
    );
    m.insert(
        "universe".into(),
        Param::int(0)
            .with_label("Universe")
            .with_range(0.0, 32767.0),
    );
    m.insert(
        "start".into(),
        Param::int(1)
            .with_label("Start Channel")
            .with_range(1.0, 512.0),
    );
    m.insert(
        "channels".into(),
        Param::int(64).with_label("Channels").with_range(1.0, 512.0),
    );
    m.insert("active".into(), Param::bool(true).with_label("Active"));
    m
}

pub(crate) fn cook_dmx_in(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let path = c.path.to_string();

    if !c.b("active") {
        c.io.dmx_in.remove(&path);
        return ChopData::new(vec![Channel::constant("chan1", 0.0, n)], rate, true);
    }

    let sacn = c.menu("protocol") == 1;
    let universe = c.i("universe").clamp(0, 32767) as u16;
    let start = c.i("start").clamp(1, 512) as usize;
    let count = c.i("channels").clamp(1, 512) as usize;
    let local = c.s("address");
    let key = dmx_listener_key(sacn, universe, &local);

    let needs_open = c.io.dmx_in.get(&path).map(|d| d.key != key).unwrap_or(true);
    if needs_open {
        c.io.dmx_in.remove(&path);
        match DmxIn::open(sacn, universe, &local) {
            Ok(d) => {
                c.io.clear_note(&path);
                c.io.dmx_in.insert(path.clone(), d);
            }
            Err(e) => c.io.note(&path, e),
        }
    }

    let mut channels = Vec::with_capacity(count);
    if let Some(dmx) = c.io.dmx_in.get(&path) {
        if let Ok(slots) = dmx.slots.lock() {
            for i in 0..count {
                let slot = start - 1 + i;
                let value = if slot < slots.len() {
                    crate::dmx::from_slot(slots[slot])
                } else {
                    0.0
                };
                channels.push(Channel::constant(format!("chan{}", i + 1), value, n));
            }
        }
    }
    if channels.is_empty() {
        channels.push(Channel::constant("chan1", 0.0, n));
    }
    ChopData::new(channels, rate, true)
}

