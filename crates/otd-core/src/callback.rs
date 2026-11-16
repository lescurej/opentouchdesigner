use std::cell::{Cell, RefCell};

use crate::expr::ChannelSource;

type NetPtr = *const (dyn ChannelSource + 'static);

thread_local! {
    static NETWORK: Cell<Option<NetPtr>> = const { Cell::new(None) };
    static NODE_PATH: RefCell<String> = const { RefCell::new(String::new()) };
}

pub struct CallbackGuard;

impl CallbackGuard {
    pub fn set(net: Option<&dyn ChannelSource>, path: &str) -> NetworkGuard {
        NODE_PATH.with(|p| *p.borrow_mut() = path.to_string());
        NETWORK.with(|n| {
            n.set(net.map(|net| unsafe {
                std::mem::transmute::<*const dyn ChannelSource, NetPtr>(net as *const dyn ChannelSource)
            }))
        });
        NetworkGuard
    }
}

pub struct NetworkGuard;

impl Drop for NetworkGuard {
    fn drop(&mut self) {
        NETWORK.with(|n| n.set(None));
    }
}

pub fn with_callback_network<R>(
    net: Option<&dyn ChannelSource>,
    path: &str,
    f: impl FnOnce() -> R,
) -> R {
    NODE_PATH.with(|p| *p.borrow_mut() = path.to_string());
    NETWORK.with(|n| {
        n.set(net.map(|net| unsafe {
            std::mem::transmute::<*const dyn ChannelSource, NetPtr>(net as *const dyn ChannelSource)
        }))
    });
    let _guard = NetworkGuard;
    f()
}

pub fn with_network<R>(f: impl FnOnce(&dyn ChannelSource, &str) -> R) -> Option<R> {
    let ptr = NETWORK.with(|n| n.get())?;
    let path = NODE_PATH.with(|p| p.borrow().clone());
    let net: &dyn ChannelSource = unsafe { &*ptr };
    Some(f(net, &path))
}
