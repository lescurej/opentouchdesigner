use std::cell::RefCell;

use crate::cook::CookEngine;

pub type CookFlush = Box<dyn FnMut(&[String]) -> Result<(), String>>;

pub struct CookSession {
    engine: *const CookEngine,
    flush: CookFlush,
}

impl CookSession {
    pub fn new(engine: &CookEngine, flush: CookFlush) -> Self {
        CookSession {
            engine: engine as *const CookEngine,
            flush,
        }
    }
}

thread_local! {
    static ACTIVE: RefCell<Option<CookSession>> = const { RefCell::new(None) };
}

pub fn cook_engine() -> Option<&'static CookEngine> {
    ACTIVE.with(|slot| {
        let guard = slot.borrow();
        let session = guard.as_ref()?;
        Some(unsafe { &*session.engine })
    })
}

pub fn run_with_cook_session<R>(session: CookSession, f: impl FnOnce() -> R) -> R {
    ACTIVE.with(|slot| {
        debug_assert!(slot.borrow().is_none(), "nested cook sessions");
        *slot.borrow_mut() = Some(session);
        let out = f();
        slot.borrow_mut().take();
        out
    })
}

pub fn flush_sync(paths: &[String]) -> Result<(), String> {
    ACTIVE.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(session) = guard.as_mut() else {
            return Err("sync recook outside cook frame".into());
        };
        (session.flush)(paths)
    })
}

pub fn in_cook_session() -> bool {
    ACTIVE.with(|slot| slot.borrow().is_some())
}
