use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use tungstenite::protocol::{Message, Role};
use tungstenite::{stream::MaybeTlsStream, WebSocket};

use super::http::push_event;
use super::types::{OutboundWs, WebSocketEvent};

pub(crate) fn spawn_ws_client(
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
