use std::collections::HashMap;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use otd_core;

use super::http::{
    bind_addr, header_value, is_websocket_upgrade, read_http_request, split_uri,
    write_http_response, write_ws_handshake,
};
use super::types::{HttpJob, OutboundWs, WebSocketEvent, WebServer};
use super::ws::spawn_ws_client;

pub(crate) fn handle_connection(
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
        spawn_ws_client(stream, client_address, uri, ws_events, ws_clients, stop);
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

        listener
            .set_nonblocking(true)
            .map_err(|e| format!("set_nonblocking: {e}"))?;
        let handle = thread::Builder::new()
            .name(format!("otd-http-{}", bound.port()))
            .spawn(move || {
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok((stream, peer)) = listener.accept() else {
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    };
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
            listen_port: port,
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
