use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

use base64::Engine;
use httparse::Status;
use sha1::{Digest, Sha1};

use super::types::{HttpReply, ParsedHttp, WebSocketEvent};

pub(crate) fn bind_addr(port: u16, local: &str) -> Result<SocketAddr, String> {
    let local = local.trim();
    if local.is_empty() {
        return Ok(SocketAddr::from(([0, 0, 0, 0], port)));
    }
    format!("{local}:{port}")
        .parse()
        .map_err(|_| format!("bad local address `{local}:{port}`"))
}

pub(crate) fn split_uri(uri: &str) -> (String, HashMap<String, String>) {
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

pub(crate) fn percent_decode(s: &str) -> String {
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

pub(crate) fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

pub(crate) fn read_http_request(stream: &mut TcpStream) -> Result<ParsedHttp, String> {
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

pub(crate) fn ws_accept_key(sec_key: &str) -> String {
    const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut hasher = Sha1::new();
    hasher.update(sec_key.as_bytes());
    hasher.update(GUID.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

pub(crate) fn is_websocket_upgrade(headers: &[(String, String)]) -> bool {
    let upgrade = header_value(headers, "upgrade").unwrap_or("");
    let connection = header_value(headers, "connection").unwrap_or("");
    upgrade.eq_ignore_ascii_case("websocket")
        && connection.to_ascii_lowercase().contains("upgrade")
        && header_value(headers, "sec-websocket-key").is_some()
}

pub(crate) fn write_ws_handshake(stream: &mut TcpStream, sec_key: &str) -> Result<(), String> {
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

pub(crate) fn write_http_response(stream: &mut TcpStream, reply: &HttpReply) -> Result<(), String> {
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

pub(crate) fn push_event(ws_events: &Arc<Mutex<Vec<WebSocketEvent>>>, ev: WebSocketEvent) {
    if let Ok(mut q) = ws_events.lock() {
        q.push(ev);
    }
}
