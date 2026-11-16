use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use otd_core::stop_io;

use crate::DatData;
use crate::net::Net;

pub const KEEP_CAP: usize = 500;

#[derive(Clone, Debug)]
pub struct TcpLine {
    pub message: String,
    pub peer: String,
}

pub struct TcpClient {
    pub inbox: Arc<Mutex<Vec<TcpLine>>>,
    pub display: Vec<TcpLine>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    pub key: String,
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

pub struct TcpServer {
    pub inbox: Arc<Mutex<Vec<TcpLine>>>,
    pub display: Vec<TcpLine>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    pub port: u16,
    pub key: String,
}

impl Drop for TcpServer {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

pub fn client_key(host: &str, port: u16) -> String {
    format!("client\0{host}\0{port}")
}

pub fn server_key(port: u16, local: &str) -> String {
    format!("server\0{local}\0{port}")
}

impl TcpClient {
    pub fn connect(host: &str, port: u16) -> Result<Self, String> {
        let addr: SocketAddr = format!("{}:{port}", host.trim())
            .parse()
            .map_err(|_| format!("bad TCP address `{host}:{port}`"))?;
        let stream = TcpStream::connect(addr).map_err(|e| format!("TCP connect: {e}"))?;
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| e.to_string())?;
        let inbox: Arc<Mutex<Vec<TcpLine>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = inbox.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let peer = addr.to_string();
        let mut reader = stream.try_clone().map_err(|e| format!("TCP clone: {e}"))?;
        let thread = std::thread::Builder::new()
            .name(format!("otd-tcp-client-{port}"))
            .spawn(move || {
                let mut buf = [0u8; 4096];
                let mut carry = String::new();
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok(n) = reader.read(&mut buf) else {
                        break;
                    };
                    if n == 0 {
                        break;
                    }
                    carry.push_str(&String::from_utf8_lossy(&buf[..n]));
                    while let Some(pos) = carry.find('\n') {
                        let line: String = carry.drain(..=pos).collect();
                        let line = line.trim_end_matches(['\r', '\n']).to_string();
                        if line.is_empty() {
                            continue;
                        }
                        push_line(&sink, line, peer.clone());
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(TcpClient {
            inbox,
            display: Vec::new(),
            stop,
            _thread: Some(thread),
            key: client_key(host, port),
        })
    }

    pub fn send(&self, payload: &[u8]) -> Result<(), String> {
        let _ = payload;
        Err("TCP client send is not wired in this build".into())
    }
}

impl TcpServer {
    pub fn bind(port: u16, local: &str) -> Result<Self, String> {
        let addr = if local.trim().is_empty() {
            format!("0.0.0.0:{port}")
        } else {
            format!("{}:{port}", local.trim())
        };
        let listener = TcpListener::bind(&addr).map_err(|e| format!("TCP bind {addr}: {e}"))?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
        let inbox: Arc<Mutex<Vec<TcpLine>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = inbox.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name(format!("otd-tcp-server-{bound_port}"))
            .spawn(move || {
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok((mut stream, peer)) = listener.accept() else {
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    };
                    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
                    let sink = sink.clone();
                    let peer_stop = stop_flag.clone();
                    let _ = std::thread::Builder::new()
                        .name("otd-tcp-peer".into())
                        .spawn(move || {
                            let mut buf = [0u8; 4096];
                            let mut carry = String::new();
                            let peer = peer.to_string();
                            while !peer_stop.load(Ordering::Relaxed) {
                                let Ok(n) = stream.read(&mut buf) else {
                                    break;
                                };
                                if n == 0 {
                                    break;
                                }
                                carry.push_str(&String::from_utf8_lossy(&buf[..n]));
                                while let Some(pos) = carry.find('\n') {
                                    let line: String = carry.drain(..=pos).collect();
                                    let line = line.trim_end_matches(['\r', '\n']).to_string();
                                    if line.is_empty() {
                                        continue;
                                    }
                                    push_line(&sink, line, peer.clone());
                                }
                            }
                        });
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(TcpServer {
            inbox,
            display: Vec::new(),
            stop,
            _thread: Some(thread),
            port: bound_port,
            key: server_key(bound_port, local),
        })
    }
}

fn push_line(sink: &Arc<Mutex<Vec<TcpLine>>>, message: String, peer: String) {
    let Ok(mut inbox) = sink.lock() else {
        return;
    };
    inbox.push(TcpLine { message, peer });
    if inbox.len() > KEEP_CAP {
        let excess = inbox.len() - KEEP_CAP;
        inbox.drain(..excess);
    }
}

pub fn empty_table() -> DatData {
    DatData::table(vec![vec!["message".into(), "peer".into()]])
}

pub fn build_table(rows: &[TcpLine], maxlines: usize) -> DatData {
    let mut out = vec![vec!["message".into(), "peer".into()]];
    let start = rows.len().saturating_sub(maxlines);
    for r in &rows[start..] {
        out.push(vec![r.message.clone(), r.peer.clone()]);
    }
    DatData::table(out)
}

pub fn send_line(host: &str, port: u16, line: &str) -> Result<(), String> {
    let addr: SocketAddr = format!("{}:{port}", host.trim())
        .parse()
        .map_err(|_| format!("bad TCP address `{host}:{port}`"))?;
    let mut stream = TcpStream::connect(addr).map_err(|e| format!("TCP connect: {e}"))?;
    let mut payload = line.as_bytes().to_vec();
    if !payload.ends_with(b"\n") {
        payload.push(b'\n');
    }
    stream
        .write_all(&payload)
        .map_err(|e| format!("TCP write: {e}"))?;
    Ok(())
}

pub fn log_rows(net: &mut Net, frame: i64, source: &str, rows: &[TcpLine]) {
    for r in rows {
        net.monitor
            .push(frame, source, "tcp", &format!("{} ({})", r.message, r.peer));
    }
}
