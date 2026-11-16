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

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::provider::{Provider, Request};

use super::parse::{binary, not_found, override_var, parse_claude, parse_codex, tail};

/// How often the parent looks to see whether the child has finished. Small
/// enough to not add latency worth measuring to a multi-second call, large
/// enough that the loop is free.
const POLL: Duration = Duration::from_millis(20);

/// Run a request through the provider's CLI and return the assistant's text.
///
/// Blocking, and slower to start than an HTTP call — process spawn plus the
/// CLI's own preamble is a second or two before any token moves. Call it from
/// a worker thread, which is what [`crate::complete_with_repair`] does.
pub fn complete(request: &Request) -> Result<String, String> {
    let provider = request.provider;
    let bin = binary(provider).ok_or_else(|| not_found(provider))?;

    let mut command = Command::new(&bin);
    let stdin_text = match provider {
        Provider::ClaudeCode => {
            command.args([
                // Non-interactive. Both formats are stream-json — see
                // `claude_turn` for why it is that rather than the simpler
                // `--output-format json`.
                "-p",
                "--input-format",
                "stream-json",
                "--output-format",
                "stream-json",
                // Which stream-json output insists on, and which costs us
                // nothing: the extra lines are ignored.
                "--verbose",
                // No CLAUDE.md, skills, plugins, hooks, MCP servers or custom
                // agents. Auth and model selection still work normally, which
                // is the whole reason this is `--safe-mode` and not `--bare`:
                // `--bare` refuses to read the OAuth login and demands an API
                // key, which is exactly the thing we are here to avoid.
                "--safe-mode",
                "--no-session-persistence",
                // Deny every tool. It still has none to reach for given the
                // prompt, but "none to reach for" is a property of the prompt
                // and this is a property of the process.
                "--allowed-tools",
                "",
            ]);
            command.args(["--system-prompt", &request.system]);
            claude_turn(request)
        }
        Provider::Codex => {
            command.args([
                "exec",
                // JSONL events on stdout; the reply is the last agent message.
                "--json",
                // It cannot write, and it will not refuse to start because a
                // temp directory is not a git repository.
                "--sandbox",
                "read-only",
                "--skip-git-repo-check",
                "--ephemeral",
                "--color",
                "never",
            ]);
            // Codex takes an image as a path rather than as data, so the
            // processed bytes are written out and handed over by name. The
            // original file is not used even when there is one: this way the
            // shrink applies, and a pasted image works the same as a dropped
            // one.
            if let Some(image) = &request.image {
                command.arg("-i").arg(image.write_temp()?);
            }
            // `codex exec` has no system-prompt flag, so the system prompt
            // goes in front of the user turn. `patch::system_prompt` reads as
            // instructions either way — it never relied on being a separate
            // role — and the reply is validated against the registry
            // regardless of what the model was told.
            format!("{}\n\n{}", request.system, request.user)
        }
        _ => return Err("not a CLI provider".into()),
    };

    // Only pass a model when one was asked for. An empty model means "use
    // whatever this CLI is configured to use", which is the safe default:
    // Codex hard-fails on a model name the signed-in account cannot use, and
    // we cannot know that account's list from here.
    let model = request.model.trim();
    if !model.is_empty() {
        // Spelled the same either way, which is luck rather than design.
        command.args(["--model", model]);
    }

    // Nothing of the user's is in reach. Codex takes its workspace root as a
    // flag as well, because `-C` is what it reads rather than the process's
    // own directory.
    let scratch = std::env::temp_dir();
    command.current_dir(&scratch);
    if provider == Provider::Codex {
        command.arg("-C").arg(&scratch);
        // `-` is "read the prompt from stdin", and it has to come last.
        command.arg("-");
    }

    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command
        .spawn()
        .map_err(|e| format!("could not start {}: {e}", bin.display()))?;

    // Writer and readers on their own threads. A prompt with a catalogue in
    // it is larger than a pipe buffer, so writing it all before reading any
    // output deadlocks: the child blocks writing its preamble to a full
    // stdout while we block writing the tail of the prompt to a full stdin.
    let mut stdin = child.stdin.take().expect("piped");
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(stdin_text.as_bytes());
        // Dropping it closes the pipe, which is how the child knows the
        // prompt has ended.
    });
    let mut out_pipe = child.stdout.take().expect("piped");
    let stdout = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = out_pipe.read_to_string(&mut buf);
        buf
    });
    let mut err_pipe = child.stderr.take().expect("piped");
    let stderr = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = err_pipe.read_to_string(&mut buf);
        buf
    });

    let deadline = Instant::now() + request.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => return Err(format!("lost track of {}: {e}", bin.display())),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{} did not answer within {}s",
                provider.label(),
                request.timeout.as_secs()
            ));
        }
        std::thread::sleep(POLL);
    };

    let _ = writer.join();
    let out = stdout.join().unwrap_or_default();
    let err = stderr.join().unwrap_or_default();

    if !status.success() && out.trim().is_empty() {
        // Nothing on stdout to parse, so stderr is the whole story.
        let detail = tail(&err);
        // A flag it has never heard of means the binary we found is older
        // than the flags this file was written against — which happens when a
        // machine has two installs and the stale one won. Naming the path is
        // the whole of the fix: the user can see which one it picked.
        if detail.contains("unknown option") || detail.contains("unexpected argument") {
            return Err(format!(
                "{} at {} is too old for this — update it, or point {} at a newer one ({detail})",
                provider.label(),
                bin.display(),
                override_var(provider).unwrap_or("the override"),
            ));
        }
        return Err(match detail.is_empty() {
            true => format!("{} exited with {status}", provider.label()),
            false => format!("{}: {detail}", provider.label()),
        });
    }

    match provider {
        Provider::ClaudeCode => parse_claude(&out, &err),
        _ => parse_codex(&out, &err),
    }
}

/// The user turn, as the one line of stream-json Claude Code reads.
///
/// Plain `-p "text"` cannot carry an image — there is no flag for a local
/// one — but `--input-format stream-json` takes ordinary Anthropic content
/// blocks, which can. So every turn goes out this way, image or not: one
/// format to build and one to parse beats two of each that differ only in
/// whether a picture is attached.
fn claude_turn(request: &Request) -> String {
    let content = match &request.image {
        Some(image) => serde_json::json!([
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": image.media_type(),
                    "data": image.base64(),
                },
            },
            { "type": "text", "text": request.user },
        ]),
        None => serde_json::json!([{ "type": "text", "text": request.user }]),
    };
    serde_json::json!({
        "type": "user",
        "message": { "role": "user", "content": content },
    })
    .to_string()
}
