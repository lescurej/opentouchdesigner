//! Operator reference, generated from the registry.
//!
//! PLAN.md §6 calls docs half the moat and says to write them alongside each
//! operator, "enforced in PR review". Review is a weak enforcement mechanism —
//! it works until the week somebody is busy. So the reference is *generated*
//! from the same `OpRegistry` the editor builds its menus and parameter pages
//! from, and a test fails the build when an operator or a parameter arrives
//! without prose.
//!
//! What that buys is docs that cannot drift. A renamed parameter, a changed
//! default, a new menu entry — the page says whatever the operator actually
//! does, because it is reading the operator. What it cannot generate is the
//! part that matters most, the *why* and the worked example, so those live in
//! `NOTES` here, keyed by operator, and are woven into the generated page.

mod notes;
mod pages;

pub use pages::*;


#[cfg(test)]
#[path = "../tests/docs__tests.rs"]
mod tests;
