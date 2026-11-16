//! Parameters — the four-mode system from PLAN.md §2.3.
//!
//! Every parameter is one of:
//!   * **Constant**   — a literal value the artist types
//!   * **Expression** — re-evaluated every cook (see [`crate::expr`])
//!   * **Export**     — driven by a CHOP channel (Phase 2; the plumbing is
//!     here, the CHOP side is not yet built)
//!   * **Bind**       — two-way link to another parameter (Phase 2)
//!
//! Modes are stored side by side rather than as an enum payload so that
//! flipping a parameter to Expression and back doesn't discard the constant
//! the artist had dialled in. TD behaves the same way, and it matters a lot in
//! live use.

mod types;

pub use types::*;
