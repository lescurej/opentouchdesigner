//! `otd-py` — embedded CPython for parameter expressions.
//!
//! PLAN.md §3 calls Python "non-negotiable for TD migration", and §6 flags the
//! GIL against realtime: "scripts run at a fixed phase of the frame, never
//! block render". Both shape what this crate is.
//!
//! **Expressions are pure and short.** A parameter expression may read the
//! network but never mutate it — [`Network`] is the only thing reachable from
//! Python here, and it is read-only. That keeps evaluation reentrant with the
//! cook and means a runaway expression can only be slow, not corrupting.
//!
//! **Compiled code is cached.** Compiling on every frame for every parameter
//! would dominate the frame; sources are compiled once and keyed by content,
//! so editing an expression recompiles exactly one thing.
//!
//! **Failure is never fatal.** A broken expression reports its error and the
//! parameter falls back to its constant, because a typo during a show must not
//! stop the render.

use std::collections::HashMap;
use std::ffi::CString;

use otd_core::{
    callback::{CallbackGuard, with_network},
    EvalContext, OscArg, ParamEdit, Value, encode_message,
    flush_pending_sync_recooks, script_actions, take_queued_deferred_recooks, take_queued_edits,
};
use otd_dat::DatData;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyTuple};

mod lint;
pub use lint::{LintSeverity, PythonLint};

mod expr_api;
mod queue;
mod paths;
mod op_api;
mod runtime;
mod engine;

pub(crate) use runtime::format_error;
pub use engine::PyEngine;

pub(crate) use queue::{
    after_script_invoke, dat_clear, dat_get_cell, dat_grid_mut, dat_num_cols,
    dat_num_rows, dat_set_cell, queue_dat_text, queue_recook,
};
pub(crate) use paths::resolve_py_op_path;
pub(crate) use expr_api::{ch, setpar};
pub(crate) use op_api::{
    me_op_ref, mod_op, op, par, parent, OpRef,
    py_to_value, value_to_py,
};
pub(crate) use runtime::{
    capture_stdio, drain_captured_stdout, merge_console_out, restore_stdio,
};

#[cfg(test)]
#[path = "tests/lib__tests.rs"]
mod tests;
