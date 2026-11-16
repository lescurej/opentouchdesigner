//! The two providers that are not a URL: Claude Code and Codex, run as
//! subprocesses against the login already on this machine.
//!
//! The point is billing. A Claude Pro/Max or ChatGPT Plus/Pro subscription
//! entitles you to *those tools*, not to an API key, and the supported way to
//! use them from another program is their own non-interactive mode — `claude
//! -p`, `codex exec`. So that is what this does: spawn the binary the user
//! already logged in with, put the prompt on its stdin, read the reply off
//! its stdout. No token is read out of anybody's credential store and no
//! request is made to an API endpoint on a subscription's behalf, because
//! both of those are somebody else's terms of service.
//!
//! Four things this file is careful about, all of them learned the hard way:
//!
//! **The binary is usually not on `PATH`.** A `.app` launched from Finder
//! inherits `/usr/bin:/bin:/usr/sbin:/sbin` and nothing else, and both of
//! these install to `~/.local/bin`. Looking only at `PATH` means the feature
//! works from a terminal and is invisible to every user who double-clicks the
//! icon. See [`binary`].
//!
//! **The agent is turned off.** Both CLIs are coding agents by default: they
//! read files, run commands, and load whatever `CLAUDE.md`/`AGENTS.md` is
//! lying around. We want one JSON object back, so tools are denied, the
//! sandbox is read-only, customisations are off, and the working directory is
//! a temp dir with nothing in it. A patch generator that can read the user's
//! home directory is not a patch generator, it is a liability.
//!
//! **Nothing is left behind.** `--no-session-persistence`/`--ephemeral`: a
//! prompt box that quietly writes a transcript for every ask is a surprise.
//!
//! **A hung child gets killed.** [`Request::timeout`] is enforced here, the
//! same as it is on the HTTP path, because a subprocess that never exits
//! holds a worker thread for the rest of the session.
//!
//! One cost worth knowing before you wonder where your rate limit went:
//! Claude Code sends its tool schemas and preamble whether or not the tools
//! are allowed, so a trivial ask still bills around 28k input tokens. It is
//! free in money and not free in weekly quota.

mod parse;
mod run;

pub use parse::*;
pub use run::*;
