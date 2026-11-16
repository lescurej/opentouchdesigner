use std::cell::RefCell;

use crate::edit::ParamEdit;
use crate::value::Value;

#[derive(Default)]
pub struct ScriptQueue {
    pub edits: Vec<ParamEdit>,
    pub recooks_deferred: Vec<String>,
    pub recooks_sync: Vec<String>,
    pub udp_sends: Vec<(String, Vec<u8>)>,
    pub ws_sends: Vec<WebSocketSend>,
}

#[derive(Clone, Debug)]
pub enum WebSocketSend {
    Text {
        server_path: String,
        client: String,
        data: String,
    },
    Binary {
        server_path: String,
        client: String,
        data: Vec<u8>,
    },
    Pong {
        server_path: String,
        client: String,
        data: Vec<u8>,
    },
}

impl ScriptQueue {
    pub fn queue_param(&mut self, path: impl Into<String>, param: impl Into<String>, value: Value) {
        self.edits.push(ParamEdit::new(path, param, value));
    }

    pub fn queue_recook(&mut self, path: impl Into<String>, deferred: bool) {
        let path = path.into();
        if deferred {
            self.recooks_deferred.push(path);
        } else {
            self.recooks_sync.push(path);
        }
    }

    pub fn queue_udp_send(&mut self, path: impl Into<String>, payload: Vec<u8>) {
        self.udp_sends.push((path.into(), payload));
    }

    pub fn take_edits(&mut self) -> Vec<ParamEdit> {
        std::mem::take(&mut self.edits)
    }

    pub fn take_deferred_recooks(&mut self) -> Vec<String> {
        std::mem::take(&mut self.recooks_deferred)
    }

    pub fn take_sync_recooks(&mut self) -> Vec<String> {
        std::mem::take(&mut self.recooks_sync)
    }

    pub fn take_ws_sends(&mut self) -> Vec<WebSocketSend> {
        std::mem::take(&mut self.ws_sends)
    }

    pub fn queue_ws_send(&mut self, msg: WebSocketSend) {
        self.ws_sends.push(msg);
    }
}

pub trait ScriptActions {
    fn queue_param(&self, path: &str, param: &str, value: Value);
    fn queue_dat_text(&self, path: &str, content_param: &str, text: String);
    fn run_op(&self, path: &str, deferred: bool);
    fn udp_send(&self, path: &str, payload: &[u8]) -> Result<(), String>;
    fn ws_send_text(&self, server_path: &str, client: &str, data: &str) -> Result<(), String>;
    fn ws_send_binary(&self, server_path: &str, client: &str, data: &[u8]) -> Result<(), String>;
    fn ws_send_pong(&self, server_path: &str, client: &str, data: &[u8]) -> Result<(), String>;
}

thread_local! {
    static SCRIPT_QUEUE: RefCell<ScriptQueue> = RefCell::new(ScriptQueue::default());
}

struct QueueScriptActions;

impl ScriptActions for QueueScriptActions {
    fn queue_param(&self, path: &str, param: &str, value: Value) {
        SCRIPT_QUEUE.with(|q| q.borrow_mut().queue_param(path, param, value));
    }

    fn queue_dat_text(&self, path: &str, content_param: &str, text: String) {
        SCRIPT_QUEUE.with(|q| {
            q.borrow_mut()
                .queue_param(path, content_param, Value::Str(text));
        });
    }

    fn run_op(&self, path: &str, deferred: bool) {
        SCRIPT_QUEUE.with(|q| q.borrow_mut().queue_recook(path, deferred));
    }

    fn udp_send(&self, path: &str, payload: &[u8]) -> Result<(), String> {
        SCRIPT_QUEUE.with(|q| {
            q.borrow_mut().queue_udp_send(path, payload.to_vec());
        });
        Ok(())
    }

    fn ws_send_text(&self, server_path: &str, client: &str, data: &str) -> Result<(), String> {
        SCRIPT_QUEUE.with(|q| {
            q.borrow_mut().queue_ws_send(WebSocketSend::Text {
                server_path: server_path.to_string(),
                client: client.to_string(),
                data: data.to_string(),
            });
        });
        Ok(())
    }

    fn ws_send_binary(&self, server_path: &str, client: &str, data: &[u8]) -> Result<(), String> {
        SCRIPT_QUEUE.with(|q| {
            q.borrow_mut().queue_ws_send(WebSocketSend::Binary {
                server_path: server_path.to_string(),
                client: client.to_string(),
                data: data.to_vec(),
            });
        });
        Ok(())
    }

    fn ws_send_pong(&self, server_path: &str, client: &str, data: &[u8]) -> Result<(), String> {
        SCRIPT_QUEUE.with(|q| {
            q.borrow_mut().queue_ws_send(WebSocketSend::Pong {
                server_path: server_path.to_string(),
                client: client.to_string(),
                data: data.to_vec(),
            });
        });
        Ok(())
    }
}

pub fn script_actions() -> &'static dyn ScriptActions {
    &QueueScriptActions
}

pub fn take_queued_edits() -> Vec<ParamEdit> {
    SCRIPT_QUEUE.with(|q| q.borrow_mut().take_edits())
}

pub fn take_queued_deferred_recooks() -> Vec<String> {
    SCRIPT_QUEUE.with(|q| q.borrow_mut().take_deferred_recooks())
}

pub fn take_queued_udp_sends() -> Vec<(String, Vec<u8>)> {
    SCRIPT_QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        std::mem::take(&mut q.udp_sends)
    })
}

pub fn take_queued_ws_sends_for(server_path: &str) -> Vec<WebSocketSend> {
    SCRIPT_QUEUE.with(|q| {
        let mut q = q.borrow_mut();
        let mut keep = Vec::new();
        let mut out = Vec::new();
        for msg in q.ws_sends.drain(..) {
            let path = match &msg {
                WebSocketSend::Text { server_path, .. }
                | WebSocketSend::Binary { server_path, .. }
                | WebSocketSend::Pong { server_path, .. } => server_path,
            };
            if path == server_path {
                out.push(msg);
            } else {
                keep.push(msg);
            }
        }
        q.ws_sends = keep;
        out
    })
}

pub fn flush_pending_sync_recooks() -> Result<(), String> {
    let paths = SCRIPT_QUEUE.with(|q| q.borrow_mut().take_sync_recooks());
    if paths.is_empty() {
        return Ok(());
    }
    match crate::cook_session::flush_sync(&paths) {
        Ok(()) => Ok(()),
        Err(_) => {
            for path in paths {
                SCRIPT_QUEUE.with(|q| q.borrow_mut().recooks_deferred.push(path));
            }
            Ok(())
        }
    }
}
