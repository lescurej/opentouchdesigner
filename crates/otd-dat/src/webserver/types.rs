use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

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
pub(crate) enum OutboundWs {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
}

pub(crate) struct ParsedHttp {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: String,
}
pub struct WebServer {
    pub port: u16,
    pub listen_port: u16,
    pub bind: String,
    pub local_address: String,
    pub(crate) pending: Arc<Mutex<Vec<HttpJob>>>,
    pub(crate) ws_events: Arc<Mutex<Vec<WebSocketEvent>>>,
    pub(crate) ws_clients: Arc<Mutex<HashMap<String, Sender<OutboundWs>>>>,
    pub(crate) stop: Arc<AtomicBool>,
    pub(crate) _thread: Option<thread::JoinHandle<()>>,
}

impl Drop for WebServer {
    fn drop(&mut self) {
        otd_core::stop_io(&self.stop, &mut self._thread);
    }
}
