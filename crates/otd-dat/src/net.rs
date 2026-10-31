//! Network DATs: UDP in/out and HTTP for the Web Client DAT.

use std::collections::HashMap;

use crate::udp::{UdpListener, UdpOutConnection};
use crate::webserver::WebServer;

#[derive(Default)]
pub struct Net {
    pub(crate) udp_in: HashMap<String, UdpListener>,
    pub(crate) udp_out: HashMap<String, UdpOutConnection>,
    pub(crate) webclient: HashMap<String, WebClient>,
    pub(crate) webserver: HashMap<String, WebServer>,
}

impl Net {
    pub fn reset(&mut self) {
        self.udp_in.clear();
        self.udp_out.clear();
        self.webclient.clear();
        self.webserver.clear();
    }
}

#[derive(Clone, Debug)]
struct WebClientResult {
    key: String,
    status: u16,
    body: String,
    error: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct WebClient {
    pub last_key: String,
    pub status: u16,
    pub body: String,
    pub error: Option<String>,
    pub notified_body: Option<String>,
    pub in_flight: Option<String>,
    pub fetch_gen: u64,
    result: std::sync::Arc<std::sync::Mutex<Option<WebClientResult>>>,
}

impl Default for WebClient {
    fn default() -> Self {
        Self {
            last_key: String::new(),
            status: 0,
            body: String::new(),
            error: None,
            notified_body: None,
            in_flight: None,
            fetch_gen: 0,
            result: std::sync::Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

pub(crate) fn webclient_poll(slot: &mut WebClient) {
    let done = slot
        .result
        .lock()
        .ok()
        .and_then(|mut g| g.take());
    let Some(done) = done else {
        return;
    };
    if slot.in_flight.as_deref() != Some(done.key.as_str()) {
        return;
    }
    slot.in_flight = None;
    slot.last_key = done.key;
    slot.status = done.status;
    slot.body = done.body;
    slot.error = done.error;
}

pub(crate) fn webclient_start_fetch(
    slot: &mut WebClient,
    key: String,
    method: String,
    url: String,
    body: String,
    headers: String,
) {
    if slot.in_flight.as_ref() == Some(&key) {
        return;
    }
    slot.in_flight = Some(key.clone());
    let sink = slot.result.clone();
    let _ = std::thread::Builder::new()
        .name("otd-webclient".into())
        .spawn(move || {
            let packed = match http_fetch(&method, &url, &body, &headers) {
                Ok((status, text)) => WebClientResult {
                    key,
                    status,
                    body: text,
                    error: None,
                },
                Err(e) => WebClientResult {
                    key,
                    status: 0,
                    body: String::new(),
                    error: Some(e),
                },
            };
            if let Ok(mut g) = sink.lock() {
                *g = Some(packed);
            }
        });
}

fn parse_headers(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_string(), value.trim().to_string()))
        })
        .collect()
}

pub(crate) fn http_fetch(
    method: &str,
    url: &str,
    body: &str,
    headers: &str,
) -> Result<(u16, String), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("URL is empty".into());
    }
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .new_agent();
    let method = method.trim().to_uppercase();
    let header_pairs = parse_headers(headers);

    let mut response = match method.as_str() {
        "POST" => {
            let mut req = agent.post(url);
            for (name, value) in &header_pairs {
                req = req.header(name, value);
            }
            req.send(body.as_bytes())
                .map_err(|e| format!("request failed: {e}"))?
        }
        "PUT" => {
            let mut req = agent.put(url);
            for (name, value) in &header_pairs {
                req = req.header(name, value);
            }
            req.send(body.as_bytes())
                .map_err(|e| format!("request failed: {e}"))?
        }
        "DELETE" => {
            let mut req = agent.delete(url);
            for (name, value) in &header_pairs {
                req = req.header(name, value);
            }
            req.call().map_err(|e| format!("request failed: {e}"))?
        }
        _ => {
            let mut req = agent.get(url);
            for (name, value) in &header_pairs {
                req = req.header(name, value);
            }
            req.call().map_err(|e| format!("request failed: {e}"))?
        }
    };
    let status = response.status().as_u16();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("could not read response: {e}"))?;
    Ok((status, text))
}
