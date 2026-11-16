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

use std::time::Duration;


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Provider {
    Anthropic,
    OpenAi,
    OpenRouter,
    /// The `claude` CLI, on this machine's login.
    ClaudeCode,
    /// The `codex` CLI, on this machine's login.
    Codex,
}

impl Provider {
    pub const ALL: &'static [Provider] = &[
        Provider::Anthropic,
        Provider::OpenAi,
        Provider::OpenRouter,
        Provider::ClaudeCode,
        Provider::Codex,
    ];

    /// Whether this provider is paid for with an API key.
    ///
    /// `false` means it runs a CLI that is already signed in, and every piece
    /// of key handling — the config file, the env var, the password field —
    /// should skip it rather than show an empty box nobody can fill.
    pub fn needs_key(&self) -> bool {
        !matches!(self, Provider::ClaudeCode | Provider::Codex)
    }

    /// The stable name used in the config file and in the UI.
    pub fn id(&self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
            Provider::OpenRouter => "openrouter",
            Provider::ClaudeCode => "claude-code",
            Provider::Codex => "codex",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::OpenAi => "OpenAI",
            Provider::OpenRouter => "OpenRouter",
            Provider::ClaudeCode => "Claude Code",
            Provider::Codex => "Codex",
        }
    }

    pub fn parse(name: &str) -> Option<Provider> {
        Provider::ALL
            .iter()
            .copied()
            .find(|p| p.id().eq_ignore_ascii_case(name.trim()))
    }

    /// The environment variable this provider's key is conventionally in, if
    /// it takes a key at all.
    pub fn env_var(&self) -> Option<&'static str> {
        match self {
            Provider::Anthropic => Some("ANTHROPIC_API_KEY"),
            Provider::OpenAi => Some("OPENAI_API_KEY"),
            Provider::OpenRouter => Some("OPENROUTER_API_KEY"),
            Provider::ClaudeCode | Provider::Codex => None,
        }
    }

    /// The URL this provider is spoken to over, if it is spoken to over one.
    /// `None` means it is a subprocess — see [`crate::cli`].
    pub fn endpoint(&self) -> Option<&'static str> {
        match self {
            Provider::Anthropic => Some("https://api.anthropic.com/v1/messages"),
            Provider::OpenAi => Some("https://api.openai.com/v1/chat/completions"),
            Provider::OpenRouter => Some("https://openrouter.ai/api/v1/chat/completions"),
            Provider::ClaudeCode | Provider::Codex => None,
        }
    }

    /// Where a user goes to get set up. Shown next to an empty key field,
    /// because "paste your API key" is useless if you have not got one — and
    /// for the CLI providers, where to go and install the thing.
    pub fn console_url(&self) -> &'static str {
        match self {
            Provider::Anthropic => "https://console.anthropic.com/settings/keys",
            Provider::OpenAi => "https://platform.openai.com/api-keys",
            Provider::OpenRouter => "https://openrouter.ai/keys",
            Provider::ClaudeCode => "https://docs.claude.com/en/docs/claude-code/overview",
            Provider::Codex => "https://developers.openai.com/codex/cli",
        }
    }

    /// A few models worth defaulting to, newest first. Free text is still
    /// accepted — this list is a convenience, not a whitelist, because it
    /// will be out of date before anybody reads it.
    pub fn models(&self) -> &'static [&'static str] {
        match self {
            Provider::Anthropic => &[
                "claude-sonnet-4-5-20250929",
                "claude-opus-4-1-20250805",
                "claude-haiku-4-5-20251001",
            ],
            Provider::OpenAi => &["gpt-5", "gpt-5-mini", "gpt-4.1", "gpt-4o"],
            Provider::OpenRouter => &[
                "anthropic/claude-sonnet-4.5",
                "openai/gpt-5",
                "google/gemini-2.5-pro",
                "meta-llama/llama-4-maverick",
            ],
            // Aliases rather than dated names: the CLI resolves them to
            // whatever is current, which is the point of using it.
            Provider::ClaudeCode => &["sonnet", "opus", "haiku"],
            // Empty first, and it means "whatever this CLI is configured to
            // use". Not politeness — Codex hard-fails on a model the
            // signed-in account has no access to, and which models those are
            // depends on the plan, so guessing a default breaks first use for
            // somebody. The CLI already knows; let it answer.
            Provider::Codex => &["", "gpt-5.1-codex", "gpt-5"],
        }
    }

    pub fn default_model(&self) -> &'static str {
        self.models()[0]
    }
}

/// One completion request. No conversation history: every ask is
/// self-contained, because the thing being edited — the patch — is the state,
/// and it is already in the prompt.
#[derive(Clone, Debug)]
pub struct Request {
    pub provider: Provider,
    pub model: String,
    pub system: String,
    pub user: String,
    /// A reference image to work from, if there is one. See [`crate::vision`].
    pub image: Option<crate::vision::Image>,
    pub max_tokens: u32,
    pub timeout: Duration,
}

impl Request {
    pub fn new(provider: Provider, model: impl Into<String>) -> Request {
        Request {
            provider,
            model: model.into(),
            system: String::new(),
            user: String::new(),
            image: None,
            max_tokens: 8192,
            // Long enough for a reasoning model on a cold start, short enough
            // that a hung provider does not leave a thread waiting all show.
            timeout: Duration::from_secs(120),
        }
    }

    pub fn system(mut self, text: impl Into<String>) -> Request {
        self.system = text.into();
        self
    }

    pub fn user(mut self, text: impl Into<String>) -> Request {
        self.user = text.into();
        self
    }

    /// Attach a reference image to work from.
    pub fn image(mut self, image: impl Into<Option<crate::vision::Image>>) -> Request {
        self.image = image.into();
        self
    }

    /// The user turn as content blocks, in whichever dialect was asked for.
    ///
    /// The image goes *before* the text in both. Anthropic say so outright,
    /// and it is the better order anyway: the instructions that follow are
    /// about the picture, and a model that has not seen it yet is being asked
    /// to remember a brief rather than apply one.
    fn content(&self, anthropic: bool) -> serde_json::Value {
        let Some(image) = &self.image else {
            return serde_json::Value::String(self.user.clone());
        };
        let block = match anthropic {
            true => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": image.media_type(),
                    "data": image.base64(),
                },
            }),
            false => serde_json::json!({
                "type": "image_url",
                "image_url": { "url": image.data_uri() },
            }),
        };
        serde_json::json!([block, { "type": "text", "text": self.user }])
    }

    /// The JSON body for this provider.
    pub fn body(&self) -> serde_json::Value {
        match self.provider {
            Provider::Anthropic => serde_json::json!({
                "model": self.model,
                "max_tokens": self.max_tokens,
                "system": self.system,
                "messages": [{ "role": "user", "content": self.content(true) }],
            }),
            // No wire body: these are subprocesses, and the prompt goes on
            // stdin. See `cli::complete`.
            Provider::ClaudeCode | Provider::Codex => serde_json::Value::Null,
            // OpenAI and OpenRouter: the system prompt is a message.
            Provider::OpenAi | Provider::OpenRouter => serde_json::json!({
                "model": self.model,
                "max_completion_tokens": self.max_tokens,
                "messages": [
                    { "role": "system", "content": self.system },
                    { "role": "user", "content": self.content(false) },
                ],
            }),
        }
    }
}

