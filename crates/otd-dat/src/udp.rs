use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use otd_core::Node;

use crate::DatData;

pub const KEEP_CAP: usize = 1000;

#[derive(Clone, Debug)]
pub struct UdpDatagram {
    pub address: String,
    pub port: u16,
    pub message: String,
    pub bytes: Vec<u8>,
}

pub struct UdpListener {
    socket: UdpSocket,
    pub inbox: Arc<Mutex<Vec<UdpDatagram>>>,
    pub pending: Arc<Mutex<Vec<UdpDatagram>>>,
    stop: Arc<AtomicBool>,
    pub port: u16,
    pub local_address: String,
}

impl Drop for UdpListener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

fn bind_addr(port: u16, local: &str) -> Result<SocketAddr, String> {
    let local = local.trim();
    if local.is_empty() {
        return Ok(SocketAddr::from(([0, 0, 0, 0], port)));
    }
    format!("{local}:{port}")
        .parse()
        .map_err(|_| format!("bad local address `{local}:{port}`"))
}

impl UdpListener {
    pub fn bind(port: u16, local: &str) -> Result<UdpListener, String> {
        let addr = bind_addr(port, local)?;
        let socket = UdpSocket::bind(addr).map_err(|e| format!("UDP bind {addr}: {e}"))?;
        socket
            .set_read_timeout(Some(std::time::Duration::from_millis(200)))
            .map_err(|e| e.to_string())?;
        let bound_port = socket.local_addr().map(|a| a.port()).unwrap_or(port);

        let inbox: Arc<Mutex<Vec<UdpDatagram>>> = Arc::new(Mutex::new(Vec::new()));
        let pending: Arc<Mutex<Vec<UdpDatagram>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = inbox.clone();
        let pending_sink = pending.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let recv = socket
            .try_clone()
            .map_err(|e| format!("UDP socket clone: {e}"))?;

        std::thread::Builder::new()
            .name(format!("otd-udp-{bound_port}"))
            .spawn(move || {
                let mut buf = [0u8; 65536];
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok((len, from)) = recv.recv_from(&mut buf) else {
                        continue;
                    };
                    let raw = buf[..len].to_vec();
                    let text = String::from_utf8_lossy(&raw)
                        .trim_end_matches(['\r', '\n'])
                        .to_string();
                    let dg = UdpDatagram {
                        address: from.ip().to_string(),
                        port: from.port(),
                        message: text,
                        bytes: raw,
                    };
                    let Ok(mut inbox) = sink.lock() else {
                        break;
                    };
                    inbox.push(dg.clone());
                    if inbox.len() > KEEP_CAP {
                        let excess = inbox.len() - KEEP_CAP;
                        inbox.drain(..excess);
                    }
                    drop(inbox);
                    if let Ok(mut pend) = pending_sink.lock() {
                        pend.push(dg);
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(UdpListener {
            socket,
            inbox,
            pending,
            stop,
            port: bound_port,
            local_address: local.to_string(),
        })
    }

    pub fn send_to(&self, payload: &[u8], target: SocketAddr) -> Result<usize, String> {
        self.socket
            .send_to(payload, target)
            .map_err(|e| format!("UDP send: {e}"))
    }
}

pub struct UdpOutConnection {
    pub listener: UdpListener,
    pub target: SocketAddr,
    pub last_sent: Option<String>,
}

impl UdpOutConnection {
    pub fn open(target: SocketAddr, local: &str) -> Result<UdpOutConnection, String> {
        let listener = UdpListener::bind(0, local)?;
        Ok(UdpOutConnection {
            listener,
            target,
            last_sent: None,
        })
    }
}

pub fn maxlines_from_node(node: &Node) -> usize {
    let v = node
        .param("maxlines")
        .or_else(|| node.param("keep"))
        .map(|p| p.value.as_i64())
        .unwrap_or(20);
    v.clamp(1, KEEP_CAP as i64) as usize
}

pub fn expand_datagram(d: &UdpDatagram, format: usize) -> Vec<UdpDatagram> {
    match format {
        1 => d
            .message
            .split(['\n', '\r', '\0'])
            .filter(|s| !s.is_empty())
            .map(|line| UdpDatagram {
                address: d.address.clone(),
                port: d.port,
                message: line.to_string(),
                bytes: line.as_bytes().to_vec(),
            })
            .collect(),
        2 => d
            .bytes
            .iter()
            .map(|b| UdpDatagram {
                address: d.address.clone(),
                port: d.port,
                message: b.to_string(),
                bytes: vec![*b],
            })
            .collect(),
        _ => vec![d.clone()],
    }
}

pub fn build_table(
    inbox: &[UdpDatagram],
    max_datagrams: usize,
    format: usize,
    bytes_col: bool,
) -> DatData {
    let start = inbox.len().saturating_sub(max_datagrams);
    let mut expanded = Vec::new();
    for d in &inbox[start..] {
        expanded.extend(expand_datagram(d, format));
    }
    let mut rows = vec![if bytes_col {
        vec![
            "address".into(),
            "port".into(),
            "message".into(),
            "bytes".into(),
        ]
    } else {
        vec!["address".into(), "port".into(), "message".into()]
    }];
    for d in expanded {
        let mut row = vec![d.address, d.port.to_string(), d.message];
        if bytes_col {
            row.push(bytes_display(&d.bytes));
        }
        rows.push(row);
    }
    DatData::table(rows)
}

fn bytes_display(raw: &[u8]) -> String {
    raw.iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn drain_pending(pending: &Arc<Mutex<Vec<UdpDatagram>>>) -> Vec<UdpDatagram> {
    pending
        .lock()
        .map(|mut p| std::mem::take(&mut *p))
        .unwrap_or_default()
}

pub fn peer_label(d: &UdpDatagram) -> String {
    format!("{}:{}", d.address, d.port)
}

pub fn empty_table(bytes_col: bool) -> DatData {
    build_table(&[], 0, 0, bytes_col)
}
