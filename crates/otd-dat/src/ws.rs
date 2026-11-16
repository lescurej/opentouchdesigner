use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use otd_core::stop_io;
use tungstenite::Message;

use crate::DatData;
use crate::net::Net;

pub const KEEP_CAP: usize = 500;

#[derive(Clone, Debug)]
pub struct WsRow {
    pub message: String,
}

pub struct WsClient {
    pub inbox: Arc<Mutex<Vec<WsRow>>>,
    pub display: Vec<WsRow>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    pub url: String,
    pub connected: Arc<AtomicBool>,
}

impl Drop for WsClient {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

impl WsClient {
    pub fn connect(url: &str) -> Result<Self, String> {
        let url = url.trim().to_string();
        if url.is_empty() {
            return Err("WebSocket URL is empty".into());
        }
        let inbox: Arc<Mutex<Vec<WsRow>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = inbox.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_flag = connected.clone();
        let url_thread = url.clone();
        let thread = std::thread::Builder::new()
            .name("otd-ws-client".into())
            .spawn(move || {
                let Ok((mut socket, _)) =
                    tungstenite::connect(&url_thread).map_err(|e| e.to_string())
                else {
                    return;
                };
                connected_flag.store(true, Ordering::Relaxed);
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok(msg) = socket.read() else {
                        break;
                    };
                    let text = match msg {
                        Message::Text(s) => s.to_string(),
                        Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
                        Message::Ping(_) | Message::Pong(_) => continue,
                        Message::Close(_) => break,
                        Message::Frame(_) => continue,
                    };
                    let Ok(mut inbox) = sink.lock() else {
                        break;
                    };
                    inbox.push(WsRow { message: text });
                    if inbox.len() > KEEP_CAP {
                        let excess = inbox.len() - KEEP_CAP;
                        inbox.drain(..excess);
                    }
                }
                connected_flag.store(false, Ordering::Relaxed);
            })
            .map_err(|e| e.to_string())?;
        Ok(WsClient {
            inbox,
            display: Vec::new(),
            stop,
            _thread: Some(thread),
            url,
            connected,
        })
    }

    pub fn send_text(&self, _text: &str) -> Result<(), String> {
        Err("WebSocket client send is not wired in this build".into())
    }
}

pub fn empty_table() -> DatData {
    DatData::table(vec![vec!["message".into()]])
}

pub fn build_table(rows: &[WsRow], maxlines: usize) -> DatData {
    let mut out = vec![vec!["message".into()]];
    let start = rows.len().saturating_sub(maxlines);
    for r in &rows[start..] {
        out.push(vec![r.message.clone()]);
    }
    DatData::table(out)
}

pub fn log_rows(net: &mut Net, frame: i64, source: &str, rows: &[WsRow]) {
    for r in rows {
        net.monitor.push(frame, source, "websocket", &r.message);
    }
}
