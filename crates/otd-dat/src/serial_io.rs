use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use otd_core::stop_io;

use crate::DatData;
use crate::net::Net;

pub const KEEP_CAP: usize = 500;

#[derive(Clone, Debug)]
pub struct SerialLine {
    pub message: String,
}

pub struct SerialLink {
    pub inbox: Arc<Mutex<Vec<SerialLine>>>,
    pub display: Vec<SerialLine>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
    pub key: String,
}

impl Drop for SerialLink {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

pub fn link_key(port: &str, baud: u32) -> String {
    format!("{port}\0{baud}")
}

impl SerialLink {
    pub fn open(port_name: &str, baud: u32) -> Result<Self, String> {
        let port = serialport::new(port_name.trim(), baud)
            .timeout(Duration::from_millis(200))
            .open()
            .map_err(|e| format!("serial open `{port_name}`: {e}"))?;
        let inbox: Arc<Mutex<Vec<SerialLine>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = inbox.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name(format!("otd-serial-{}", port_name.trim()))
            .spawn(move || {
                let mut port = port;
                let mut buf = [0u8; 4096];
                let mut carry = String::new();
                while !stop_flag.load(Ordering::Relaxed) {
                    let Ok(n) = port.read(&mut buf) else {
                        continue;
                    };
                    if n == 0 {
                        continue;
                    }
                    carry.push_str(&String::from_utf8_lossy(&buf[..n]));
                    while let Some(pos) = carry.find('\n') {
                        let line: String = carry.drain(..=pos).collect();
                        let line = line.trim_end_matches(['\r', '\n']).to_string();
                        if line.is_empty() {
                            continue;
                        }
                        let Ok(mut inbox) = sink.lock() else {
                            break;
                        };
                        inbox.push(SerialLine { message: line });
                        if inbox.len() > KEEP_CAP {
                            let excess = inbox.len() - KEEP_CAP;
                            inbox.drain(..excess);
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(SerialLink {
            inbox,
            display: Vec::new(),
            stop,
            _thread: Some(thread),
            key: link_key(port_name, baud),
        })
    }

    pub fn write(&self, payload: &[u8]) -> Result<(), String> {
        let _ = payload;
        Err("serial write is not wired in this build".into())
    }
}

pub fn empty_table() -> DatData {
    DatData::table(vec![vec!["message".into()]])
}

pub fn build_table(rows: &[SerialLine], maxlines: usize) -> DatData {
    let mut out = vec![vec!["message".into()]];
    let start = rows.len().saturating_sub(maxlines);
    for r in &rows[start..] {
        out.push(vec![r.message.clone()]);
    }
    DatData::table(out)
}

pub fn log_rows(net: &mut Net, frame: i64, source: &str, rows: &[SerialLine]) {
    for r in rows {
        net.monitor.push(frame, source, "serial", &r.message);
    }
}
