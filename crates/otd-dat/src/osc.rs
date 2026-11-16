use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use otd_core::stop_io;
use rosc::OscType;

use crate::DatData;
use crate::net::Net;

pub const KEEP_CAP: usize = 1000;

#[derive(Clone, Debug)]
pub struct OscRow {
    pub address: String,
    pub message: String,
    pub peer: String,
}

pub struct OscListener {
    pub inbox: Arc<Mutex<Vec<OscRow>>>,
    pub display: Vec<OscRow>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    pub port: u16,
    pub local_address: String,
    pub key: String,
}

impl Drop for OscListener {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

pub fn listener_key(port: u16, local: &str, multicast: bool, net: &str) -> String {
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

impl OscListener {
    pub fn open(port: u16, local: &str, multicast: bool, net: &str) -> Result<Self, String> {
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
        let bound_port = socket.local_addr().map(|a| a.port()).unwrap_or(port);
        let key = listener_key(bound_port, local, multicast, net);
        let inbox: Arc<Mutex<Vec<OscRow>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = inbox.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let recv = socket
            .try_clone()
            .map_err(|e| format!("OSC socket clone: {e}"))?;

        let thread = std::thread::Builder::new()
            .name(format!("otd-osc-dat-{bound_port}"))
            .spawn(move || {
                let mut buf = [0u8; 65536];
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok((len, from)) = recv.recv_from(&mut buf) else {
                        continue;
                    };
                    let Ok((_, packet)) = rosc::decoder::decode_udp(&buf[..len]) else {
                        continue;
                    };
                    collect_packet(&packet, &from.to_string(), &sink);
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(OscListener {
            inbox,
            display: Vec::new(),
            stop,
            _thread: Some(thread),
            port: bound_port,
            local_address: local.to_string(),
            key,
        })
    }
}

fn format_args(args: &[OscType]) -> String {
    args.iter()
        .map(|a| match a {
            OscType::Float(f) => f.to_string(),
            OscType::Double(d) => d.to_string(),
            OscType::Int(i) => i.to_string(),
            OscType::Long(l) => l.to_string(),
            OscType::Bool(b) => b.to_string(),
            OscType::String(s) => s.clone(),
            OscType::Blob(b) => format!("<{} bytes>", b.len()),
            OscType::Midi(m) => format!("midi:{m:?}"),
            OscType::Time(t) => format!("time:{t:?}"),
            OscType::Color(c) => format!("color:{c:?}"),
            OscType::Char(c) => c.to_string(),
            OscType::Array(a) => format!("[{}]", format_args(&a.content)),
            OscType::Nil => "nil".into(),
            OscType::Inf => "inf".into(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn collect_packet(packet: &rosc::OscPacket, peer: &str, sink: &Arc<Mutex<Vec<OscRow>>>) {
    match packet {
        rosc::OscPacket::Bundle(b) => {
            for p in &b.content {
                collect_packet(p, peer, sink);
            }
        }
        rosc::OscPacket::Message(m) => {
            let row = OscRow {
                address: m.addr.clone(),
                message: format_args(&m.args),
                peer: peer.to_string(),
            };
            let Ok(mut inbox) = sink.lock() else {
                return;
            };
            inbox.push(row);
            if inbox.len() > KEEP_CAP {
                let excess = inbox.len() - KEEP_CAP;
                inbox.drain(..excess);
            }
        }
    }
}

pub fn empty_table() -> DatData {
    DatData::table(vec![vec![
        "address".into(),
        "message".into(),
        "peer".into(),
    ]])
}

pub fn build_table(rows: &[OscRow], maxlines: usize) -> DatData {
    let mut out = vec![vec!["address".into(), "message".into(), "peer".into()]];
    let start = rows.len().saturating_sub(maxlines);
    for r in &rows[start..] {
        out.push(vec![r.address.clone(), r.message.clone(), r.peer.clone()]);
    }
    DatData::table(out)
}

pub fn send_message(host: &str, port: u16, address: &str, value: f32) -> Result<(), String> {
    let addr: SocketAddr = format!("{}:{port}", host.trim())
        .parse()
        .map_err(|_| format!("bad OSC target `{host}:{port}`"))?;
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("OSC send bind: {e}"))?;
    let msg = rosc::OscMessage {
        addr: address.trim().to_string(),
        args: vec![OscType::Float(value)],
    };
    let packet = rosc::OscPacket::Message(msg);
    let buf = rosc::encoder::encode(&packet).map_err(|e| e.to_string())?;
    socket
        .send_to(&buf, addr)
        .map_err(|e| format!("OSC send: {e}"))?;
    Ok(())
}

pub fn log_rows(net: &mut Net, frame: i64, source: &str, rows: &[OscRow]) {
    for r in rows {
        net.monitor.push(
            frame,
            source,
            "osc",
            &format!("{} {}", r.address, r.message),
        );
    }
}
