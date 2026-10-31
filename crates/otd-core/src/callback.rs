use std::cell::Cell;

use crate::expr::ChannelSource;

type NetPtr = *const (dyn ChannelSource + 'static);

thread_local! {
    static NETWORK: Cell<Option<NetPtr>> = const { Cell::new(None) };
    static NODE_PATH: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

pub struct CallbackGuard;

impl CallbackGuard {
    pub fn set(net: Option<&dyn ChannelSource>, path: &str) -> CallbackGuard {
        NETWORK.with(|n| {
            n.set(net.map(|n| unsafe {
                std::mem::transmute::<*const dyn ChannelSource, NetPtr>(n as *const dyn ChannelSource)
            }))
        });
        NODE_PATH.with(|p| *p.borrow_mut() = path.to_string());
        CallbackGuard
    }
}

impl Drop for CallbackGuard {
    fn drop(&mut self) {
        NETWORK.with(|n| n.set(None));
        NODE_PATH.with(|p| p.borrow_mut().clear());
    }
}

pub fn with_network<R>(f: impl FnOnce(&dyn ChannelSource, &str) -> R) -> Option<R> {
    let ptr = NETWORK.with(|n| n.get())?;
    let path = NODE_PATH.with(|p| p.borrow().clone());
    let net: &dyn ChannelSource = unsafe { &*ptr };
    Some(f(net, &path))
}
