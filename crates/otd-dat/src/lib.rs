//! `otd-dat` — the DAT (data operator) family: text and tables.

//!
//! PLAN.md §2.1 lists DATs alongside TOPs and CHOPs as one of the load-bearing
//! wire families. They are the family that carries *structure* — a table of
//! cue points, a JSON payload from a control surface, a script.
//!
//! Two decisions worth stating. Content lives in a parameter rather than in a
//! side file, so a table is part of the project's text and shows up in a diff
//! like everything else. And a table is a grid of strings, not of typed
//! values: a DAT is text, and the operator reading it decides what the text
//! means.

pub mod callbacks;
pub mod chop_mirror;
pub mod folder;
pub mod monitor;
pub mod net;
pub mod ops;
pub mod osc;
pub mod serial_io;
pub mod tcp;
pub mod udp;
pub mod webserver;
pub mod ws;

mod dat;

pub use dat::{DatData, DatEngine, DatHost, DatNetwork, DatStore, ScriptHost};

pub fn register_all(registry: &mut otd_core::OpRegistry) {
    ops::register_all(registry);
}
