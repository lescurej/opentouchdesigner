//! The five providers, behind one call.
//!
//! Three of them are a URL and a key. OpenAI and OpenRouter speak the same
//! wire format — OpenRouter exists to be a drop-in for it — so they share a
//! request builder and differ only in host and headers. Anthropic's is close
//! but not the same: the system prompt is a top-level field rather than a
//! message, auth is `x-api-key` rather than a bearer token, and it wants a
//! version header. That is the whole difference, and it is small enough that
//! a trait would cost more than it saved.
//!
//! Two of them are not a URL at all. Claude Code and Codex are subprocesses
//! run against the login already on the machine, so somebody paying for one
//! of those subscriptions does not also have to buy API credit. Everything
//! they need is in [`crate::cli`]; [`complete`] dispatches to it and the rest
//! of the crate never learns the difference. [`Provider::needs_key`] is the
//! seam — a provider that does not need a key does not get a key field, an
//! env var, or a line in `keys.conf`.
//!
//! Everything here is blocking. The caller runs it on a worker thread, which
//! is the same rule every device in this codebase follows: nothing that can
//! wait on the network is allowed anywhere near a frame.

mod http;
mod types;

pub use http::*;
pub use types::*;
