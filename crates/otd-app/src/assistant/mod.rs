//! The assistant panel: describe a patch, get operators.
//!
//! Three things this panel is careful about.
//!
//! **The request happens on a worker thread.** A completion takes anywhere
//! from two seconds to a minute, and a frozen editor for a minute is not a
//! realtime tool. The prompt is built here, where the graph is; the HTTP call
//! happens there; the reply is validated and applied back here.
//!
//! **The result is one undo.** A checkpoint is taken before anything is
//! created, so a patch you did not want is `Cmd+Z` and gone, not nine
//! deletions.
//!
//! **The key field is not where the key lives.** It is typed here and saved
//! to a `0600` file outside every project, or it comes from the environment
//! and is never typed at all. It does not go into the `.otd`.
//!
//! Two of the five providers have no key at all: Claude Code and Codex run
//! the CLI this machine is already signed in to, so "configured" means "the
//! binary is there" rather than "a key is stored". [`Provider::needs_key`]
//! decides which panel a provider gets, and [`Assistant::ready`] is the one
//! question the picker asks either way.


mod apply;
mod bar;
mod drop;
mod labels;
mod patch;
mod state;
mod window;

pub use state::Assistant;
pub use drop::{claim_drop, would_claim};
pub use bar::bar;
pub use window::window;
pub use apply::apply_recipe;

pub(crate) use patch::send;
