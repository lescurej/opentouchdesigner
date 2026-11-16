//! Edge I/O pattern: a device thread pushes into an inbox; the cook thread drains.
//!
//! Typical wiring:
//! - `Arc<Mutex<Vec<T>>>` inbox filled by cpal, UDP, decode, etc.
//! - Cook calls [`cook_drain`] to merge into a display/hold buffer with a cap.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub fn drain_inbox<T>(inbox: &Arc<Mutex<Vec<T>>>) -> Vec<T> {
    inbox
        .lock()
        .map(|mut v| std::mem::take(&mut *v))
        .unwrap_or_default()
}

pub fn trim_front<T>(items: &mut Vec<T>, cap: usize) {
    if items.len() > cap {
        let excess = items.len() - cap;
        items.drain(..excess);
    }
}

pub fn cook_drain<T: Clone>(
    inbox: &Arc<Mutex<Vec<T>>>,
    display: &mut Vec<T>,
    cap: usize,
) -> Vec<T> {
    let batch = drain_inbox(inbox);
    display.extend(batch.iter().cloned());
    trim_front(display, cap);
    batch
}

pub fn stop_io(stop: &AtomicBool, thread: &mut Option<JoinHandle<()>>) {
    stop.store(true, Ordering::Relaxed);
    if let Some(handle) = thread.take() {
        let _ = handle.join();
    }
}

/// Shared inbox helper for edge threads.
pub struct EdgeInbox<T> {
    pub inbox: Arc<Mutex<Vec<T>>>,
    pub cap: usize,
}

impl<T: Clone> EdgeInbox<T> {
    pub fn new(cap: usize) -> Self {
        Self {
            inbox: Arc::new(Mutex::new(Vec::new())),
            cap,
        }
    }

    /// Called on the edge thread.
    pub fn push(&self, item: T) {
        if let Ok(mut v) = self.inbox.lock() {
            v.push(item);
            trim_front(&mut v, self.cap);
        }
    }

    /// Called on the cook thread.
    pub fn drain_for_cook(&self, display: &mut Vec<T>) -> Vec<T> {
        cook_drain(&self.inbox, display, self.cap)
    }
}

#[cfg(test)]
#[path = "tests/io_async__tests.rs"]
mod tests;
