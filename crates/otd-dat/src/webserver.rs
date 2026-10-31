use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use base64::Engine;
use httparse::Status;
use sha1::{Digest, Sha1};
use tungstenite::protocol::{Message, Role};
use tungstenite::{stream::MaybeTlsStream, WebSocket};

#[derive(Clone, Debug)]
pub struct HttpJob {
    pub method: String,
    pub uri: String,
    pub pars: HashMap<String, String>,
    pub client_address: String,
    pub server_address: String,
    pub data: String,
    pub(crate) respond: mpsc::Sender<HttpReply>,
}

#[derive(Clone, Debug)]
pub struct HttpReply {
    pub status: u16,
    pub reason: String,
    pub body: String,
    pub content_type: Option<String>,
}

impl Default for HttpReply {
    fn default() -> Self {
        Self {
            status: 404,
            reason: "Not Found".into(),
            body: String::new(),
            content_type: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum WebSocketEvent {
    Open { client: String, uri: String },
    Close { client: String },
    Text { client: String, data: String },
    Binary { client: String, data: Vec<u8> },
    Ping { client: String, data: Vec<u8> },
    Pong { client: String, data: Vec<u8> },
}

#[derive(Clone)]
enum OutboundWs {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
}

struct ParsedHttp {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

pub struct WebServer {
    pub port: u16,
    pub bind: String,
    pub local_address: String,
    pending: Arc<Mutex<Vec<HttpJob>>>,
    ws_events: Arc<Mutex<Vec<WebSocketEvent>>>,
    ws_clients: Arc<Mutex<HashMap<String, Sender<OutboundWs>>>>,
    stop: Arc<AtomicBool>,
    _thread: Option<thread::JoinHandle<()>>,
}

impl Drop for WebServer {
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

fn split_uri(uri: &str) -> (String, HashMap<String, String>) {
    let uri = uri.trim();
    let Some((path, query)) = uri.split_once('?') else {
        return (uri.to_string(), HashMap::new());
    };
    let mut pars = HashMap::new();
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        pars.insert(percent_decode(k), percent_decode(v));
    }
    (path.to_string(), pars)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn read_http_request(stream: &mut TcpStream) -> Result<ParsedHttp, String> {
    let mut buf = Vec::with_capacity(4096);
    loop {
        if buf.len() >= 65536 {
            return Err("HTTP request too large".into());
        }
        let mut chunk = [0u8; 2048];
        let n = stream.read(&mut chunk).map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            return Err("connection closed".into());
        }
        buf.extend_from_slice(&chunk[..n]);
        let mut header_buf = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut header_buf);
        match req.parse(&buf).map_err(|e| e.to_string())? {
            Status::Complete(len) => {
                let method = req.method.unwrap_or("GET").to_string();
                let path = req.path.unwrap_or("/").to_string();
                let headers: Vec<(String, String)> = req
                    .headers
                    .iter()
                    .filter_map(|h| {
                        Some((
                            h.name.to_string(),
                            std::str::from_utf8(h.value).ok()?.to_string(),
                        ))
                    })
                    .collect();
                let mut content_length = 0usize;
                if let Some(cl) = header_value(&headers, "content-length") {
                    content_length = cl.parse().unwrap_or(0);
                }
                while buf.len() < len + content_length {
                    let mut chunk = [0u8; 2048];
                    let n = stream.read(&mut chunk).map_err(|e| format!("read: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..n]);
                }
                let body = if content_length > 0 && buf.len() >= len + content_length {
                    String::from_utf8_lossy(&buf[len..len + content_length]).into_owned()
                } else {
                    String::new()
                };
                return Ok(ParsedHttp {
                    method,
                    path,
                    headers,
                    body,
                });
            }
            Status::Partial => continue,
        }
    }
}

fn ws_accept_key(sec_key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut hasher = Sha1::new();
    hasher.update(sec_key.as_bytes());
    hasher.update(GUID.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

fn is_websocket_upgrade(headers: &[(String, String)]) -> bool {
    let upgrade = header_value(headers, "upgrade").unwrap_or("");
    let connection = header_value(headers, "connection").unwrap_or("");
    upgrade.eq_ignore_ascii_case("websocket")
        && connection.to_ascii_lowercase().contains("upgrade")
        && header_value(headers, "sec-websocket-key").is_some()
}

fn write_ws_handshake(stream: &mut TcpStream, sec_key: &str) -> Result<(), String> {
    let accept = ws_accept_key(sec_key);
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\r\n"
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("write handshake: {e}"))
}

fn write_http_response(stream: &mut TcpStream, reply: &HttpReply) -> Result<(), String> {
    let ct = reply
        .content_type
        .as_deref()
        .unwrap_or("text/plain; charset=utf-8");
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        reply.status,
        reply.reason,
        ct,
        reply.body.len(),
        reply.body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("write response: {e}"))
}

fn push_event(ws_events: &Arc<Mutex<Vec<WebSocketEvent>>>, ev: WebSocketEvent) {
    if let Ok(mut q) = ws_events.lock() {
        q.push(ev);
    }
}

fn spawn_ws_client(
    stream: TcpStream,
    client: String,
    uri: String,
    ws_events: Arc<Mutex<Vec<WebSocketEvent>>>,
    ws_clients: Arc<Mutex<HashMap<String, Sender<OutboundWs>>>>,
    stop: Arc<AtomicBool>,
) {
    let (tx, rx) = mpsc::channel();
    if let Ok(mut clients) = ws_clients.lock() {
        clients.insert(client.clone(), tx);
    }
    push_event(
        &ws_events,
        WebSocketEvent::Open {
            client: client.clone(),
            uri,
        },
    );
    thread::Builder::new()
        .name(format!("otd-ws-{client}"))
        .spawn(move || {
            let stream = MaybeTlsStream::Plain(stream);
            let mut ws = WebSocket::from_raw_socket(stream, Role::Server, None);
            loop {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                while let Ok(msg) = rx.try_recv() {
                    let out = match msg {
                        OutboundWs::Text(s) => Message::Text(s.into()),
                        OutboundWs::Binary(b) => Message::Binary(b.into()),
                        OutboundWs::Pong(b) => Message::Pong(b.into()),
                    };
                    let _ = ws.send(out);
                }
                match ws.read() {
                    Ok(Message::Text(s)) => {
                        push_event(
                            &ws_events,
                            WebSocketEvent::Text {
                                client: client.clone(),
                                data: s.to_string(),
                            },
                        );
                    }
                    Ok(Message::Binary(b)) => {
                        push_event(
                            &ws_events,
                            WebSocketEvent::Binary {
                                client: client.clone(),
                                data: b.to_vec(),
                            },
                        );
                    }
                    Ok(Message::Ping(b)) => {
                        push_event(
                            &ws_events,
                            WebSocketEvent::Ping {
                                client: client.clone(),
                                data: b.to_vec(),
                            },
                        );
                        let _ = ws.send(Message::Pong(b));
                    }
                    Ok(Message::Pong(b)) => {
                        push_event(
                            &ws_events,
                            WebSocketEvent::Pong {
                                client: client.clone(),
                                data: b.to_vec(),
                            },
                        );
                    }
                    Ok(Message::Close(_)) | Ok(Message::Frame(_)) => break,
                    Err(_) => break,
                }
            }
            let _ = ws.close(None);
            if let Ok(mut clients) = ws_clients.lock() {
                clients.remove(&client);
            }
            push_event(
                &ws_events,
                WebSocketEvent::Close {
                    client: client.clone(),
                },
            );
        })
        .ok();
}

fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    server_address: String,
    pending: Arc<Mutex<Vec<HttpJob>>>,
    ws_events: Arc<Mutex<Vec<WebSocketEvent>>>,
    ws_clients: Arc<Mutex<HashMap<String, Sender<OutboundWs>>>>,
    stop: Arc<AtomicBool>,
) {
    let mut stream = stream;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let req = match read_http_request(&mut stream) {
        Ok(v) => v,
        Err(_) => return,
    };
    let client_address = peer.to_string();

    if is_websocket_upgrade(&req.headers) {
        let sec_key = header_value(&req.headers, "sec-websocket-key").unwrap_or("");
        let (uri, _) = split_uri(&req.path);
        if write_ws_handshake(&mut stream, sec_key).is_err() {
            return;
        }
        spawn_ws_client(
            stream,
            client_address,
            uri,
            ws_events,
            ws_clients,
            stop,
        );
        return;
    }

    let (uri, pars) = split_uri(&req.path);
    let (tx, rx) = mpsc::channel();
    let job = HttpJob {
        method: req.method,
        uri,
        pars,
        client_address,
        server_address,
        data: req.body,
        respond: tx,
    };
    if let Ok(mut q) = pending.lock() {
        q.push(job);
    }
    let reply = rx.recv_timeout(Duration::from_secs(10)).unwrap_or_default();
    let _ = write_http_response(&mut stream, &reply);
}

impl WebServer {
    pub fn start(port: u16, bind: &str) -> Result<WebServer, String> {
        let addr = bind_addr(port, bind)?;
        let listener = TcpListener::bind(addr).map_err(|e| format!("HTTP bind {addr}: {e}"))?;
        let bound = listener
            .local_addr()
            .map_err(|e| format!("local_addr: {e}"))?;
        let pending: Arc<Mutex<Vec<HttpJob>>> = Arc::new(Mutex::new(Vec::new()));
        let ws_events: Arc<Mutex<Vec<WebSocketEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let ws_clients: Arc<Mutex<HashMap<String, Sender<OutboundWs>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let queue = pending.clone();
        let ws_ev = ws_events.clone();
        let ws_cl = ws_clients.clone();
        let server_address = bound.to_string();

        let handle = thread::Builder::new()
            .name(format!("otd-http-{}", bound.port()))
            .spawn(move || {
        for conn in listener.incoming() {
                    if stop_flag.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(stream) = conn else { continue };
                    let peer = stream
                        .peer_addr()
                        .unwrap_or_else(|_| "0.0.0.0:0".parse().unwrap());
                    handle_connection(
                        stream,
                        peer,
                        server_address.clone(),
                        queue.clone(),
                        ws_ev.clone(),
                        ws_cl.clone(),
                        stop_flag.clone(),
                    );
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(WebServer {
            port: bound.port(),
            bind: bind.to_string(),
            local_address: bound.ip().to_string(),
            pending,
            ws_events,
            ws_clients,
            stop,
            _thread: Some(handle),
        })
    }

    pub fn drain_pending(&self) -> Vec<HttpJob> {
        self.pending
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default()
    }

    pub fn drain_ws_events(&self) -> Vec<WebSocketEvent> {
        self.ws_events
            .lock()
            .map(|mut q| std::mem::take(&mut *q))
            .unwrap_or_default()
    }

    fn send_ws(&self, client: &str, msg: OutboundWs) {
        if let Ok(clients) = self.ws_clients.lock() {
            if let Some(tx) = clients.get(client) {
                let _ = tx.send(msg);
            }
        }
    }

    pub fn apply_ws_sends(&self, sends: &[otd_core::WebSocketSend]) {
        for send in sends {
            match send {
                otd_core::WebSocketSend::Text { client, data, .. } => {
                    self.send_ws(client, OutboundWs::Text(data.clone()));
                }
                otd_core::WebSocketSend::Binary { client, data, .. } => {
                    self.send_ws(client, OutboundWs::Binary(data.clone()));
                }
                otd_core::WebSocketSend::Pong { client, data, .. } => {
                    self.send_ws(client, OutboundWs::Pong(data.clone()));
                }
            }
        }
    }
}

pub fn server_key(port: u16, bind: &str) -> String {
    format!("{bind}\0{port}")
}
