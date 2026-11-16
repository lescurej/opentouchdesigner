use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use otd_core::stop_io;

use crate::data::{Channel, ChopData, slice_len};
use crate::ops::ChopCtx;

pub(crate) struct SerialLink {
    port: String,
    baud: u32,
    direction: usize,
    port_handle: Box<dyn serialport::SerialPort>,
    inbox: Arc<Mutex<VecDeque<u8>>>,
    stop: Arc<AtomicBool>,
    _thread: Option<JoinHandle<()>>,
}

impl Drop for SerialLink {
    fn drop(&mut self) {
        stop_io(&self.stop, &mut self._thread);
    }
}

impl SerialLink {
    fn open(port_name: &str, baud: u32, direction: usize) -> Result<SerialLink, String> {
        let ports = serialport::available_ports().map_err(|e| e.to_string())?;
        if ports.is_empty() {
            return Err("no serial ports".into());
        }
        let port_type = if port_name.trim().is_empty() {
            ports[0].port_name.clone()
        } else {
            ports
                .iter()
                .find(|p| p.port_name.contains(port_name))
                .map(|p| p.port_name.clone())
                .ok_or_else(|| format!("no serial port matching `{port_name}`"))?
        };
        let handle = serialport::new(&port_type, baud)
            .timeout(Duration::from_millis(10))
            .open()
            .map_err(|e| format!("{port_type}: {e}"))?;
        let inbox = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let mut thread = None;
        if direction == 0 {
            let reader_inbox = inbox.clone();
            let stop_flag = stop.clone();
            let mut reader = handle.try_clone().map_err(|e| e.to_string())?;
            thread = Some(
                std::thread::Builder::new()
                    .name(format!("otd-serial-{port_type}"))
                    .spawn(move || {
                        let mut buf = [0u8; 256];
                        while !stop_flag.load(Ordering::Relaxed) {
                            match reader.read(&mut buf) {
                                Ok(0) => std::thread::sleep(Duration::from_millis(20)),
                                Ok(n) => {
                                    if let Ok(mut q) = reader_inbox.lock() {
                                        for b in &buf[..n] {
                                            q.push_back(*b);
                                            if q.len() > 4096 {
                                                q.pop_front();
                                            }
                                        }
                                    }
                                }
                                Err(_) => std::thread::sleep(Duration::from_millis(20)),
                            }
                        }
                    })
                    .map_err(|e| e.to_string())?,
            );
        }
        Ok(SerialLink {
            port: port_type,
            baud,
            direction,
            port_handle: handle,
            inbox,
            stop,
            _thread: thread,
        })
    }
}

pub fn cook_serial(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let path = c.path.to_string();
    let port = c.s("port");
    let baud = c.i("baud").clamp(300, 921_600) as u32;
    let direction = c.menu("direction");
    let active = c.b("active");

    if !active {
        c.io.serial.remove(&path);
        return ChopData::new(vec![Channel::constant("chan1", 0.0, n)], rate, true);
    }

    let needs_open =
        c.io.serial
            .get(&path)
            .map(|s| s.port != port || s.baud != baud || s.direction != direction)
            .unwrap_or(true);
    if needs_open {
        match SerialLink::open(&port, baud, direction) {
            Ok(link) => {
                c.io.clear_note(&path);
                c.io.serial.insert(path.clone(), link);
            }
            Err(e) => {
                c.io.note(&path, e);
                c.io.serial.remove(&path);
            }
        }
    }

    if direction == 1 {
        let input = c.input(0).clone();
        if let Some(link) = c.io.serial.get_mut(&path) {
            if let Some(ch) = input.channels.first() {
                for s in &ch.samples {
                    let byte = (s.clamp(0.0, 1.0) * 255.0).round() as u8;
                    let _ = link.port_handle.write_all(&[byte]);
                }
            }
        }
        return input;
    }

    let mut bytes = Vec::new();
    if let Some(link) = c.io.serial.get(&path) {
        if let Ok(mut q) = link.inbox.lock() {
            while bytes.len() < 32 {
                match q.pop_front() {
                    Some(b) => bytes.push(b),
                    None => break,
                }
            }
        }
    }
    let channels: Vec<Channel> = if bytes.is_empty() {
        vec![Channel::constant("chan1", 0.0, n)]
    } else {
        bytes
            .into_iter()
            .enumerate()
            .map(|(i, b)| Channel::constant(format!("byte{}", i + 1), b as f32 / 255.0, n))
            .collect()
    };
    ChopData::new(channels, rate, true)
}
