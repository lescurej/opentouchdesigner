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

use std::collections::BTreeMap;
use std::sync::mpsc::Receiver;

type PlanReply = Result<(String, Option<otd_ai::Repair>), String>;

use otd_ai::{Keys, Provider, cli};


pub struct Assistant {
    /// The settings window: providers, keys, what was skipped.
    pub open: bool,
    /// The floating bar over the canvas. On by default — a feature behind a
    /// menu is a feature nobody finds — and dismissable, because a bar over
    /// your work that you cannot get rid of is worse than no bar.
    pub bar: bool,
    /// Collapsed to a pill. Out of the way without being gone.
    pub collapsed: bool,
    /// Set for one frame to move the caret into the bar.
    pub(crate) focus_bar: bool,
    pub provider: Provider,
    pub model: String,
    /// What is in the key field right now. Saved on demand, never read back
    /// out of storage into the UI — a key you can re-read is a key you can
    /// screenshot.
    pub key_input: String,
    pub prompt: String,
    pub(crate) conversation: otd_ai::Conversation,
    pub(crate) submitted_prompt: String,
    /// Whether the assistant may remove operators as well as add them. Off by
    /// default: adding to a patch is recoverable by looking at it, and a node
    /// that quietly went away is not — you have to notice the absence first.
    /// With it on, "make this simpler" and "take the blur out" work.
    pub allow_delete: bool,
    /// Whether the request is confined to the selected operators. Off by
    /// default, because most asks are about the patch as a whole.
    ///
    /// On, the rest of the network is locked: nothing outside the selection is
    /// retuned, removed, or rewired. Useful once a patch is big enough that
    /// "make the trails longer" could reasonably be read as touching three
    /// different chains.
    pub scope_selection: bool,
    /// The names in scope at the moment the request went out.
    ///
    /// Captured at send rather than read at reply, because the selection is a
    /// live thing and the round trip is seconds long: clicking a different
    /// node while the model is thinking must not move the goalposts the answer
    /// is filtered against.
    pub scoped_to: Vec<String>,
    /// A still to work back from. With one attached the prompt is optional:
    /// pointing at a picture is a complete request.
    pub image: Option<otd_ai::Image>,
    /// What that image is — a reference, a camera frame, or a look at the
    /// result — which picks the brief it goes out with.
    pub seen: otd_ai::Seen,
    /// The camera loop, while one is running. See `crate::watch`.
    pub watch: Option<crate::watch::Watch>,
    /// The thumbnail, uploaded once and dropped whenever the image changes.
    /// Rebuilding a texture every frame is how a 96-pixel preview becomes a
    /// performance problem.
    pub(crate) image_tex: Option<egui::TextureHandle>,
    /// Where the bar was last drawn, so a file dropped on it is attached
    /// rather than turned into a Movie File In. See [`claim_drop`].
    pub bar_rect: egui::Rect,
    pub(crate) keys: Keys,
    /// Whether each CLI provider's binary was found. A filesystem lookup, so
    /// cheap enough to do at startup — unlike asking it its version, which
    /// spawns a process and waits for it.
    pub(crate) cli_found: BTreeMap<Provider, bool>,
    /// What `--version` said, filled in the first time the settings window
    /// draws that provider and on demand after that.
    pub(crate) cli_version: BTreeMap<Provider, Result<String, String>>,
    pub(crate) pending: Option<Receiver<PlanReply>>,
    /// Notes from the last plan that was applied.
    pub last: Option<String>,
    pub warnings: Vec<String>,
    /// Shaders that were built anyway and will not compile, as
    /// `(node, error)`. Kept apart from `warnings` because they are not the
    /// same kind of news: a skipped parameter leaves a patch that works, and
    /// a red glslTOP leaves one whose output is black from that node down.
    /// Folded into "N thing(s) skipped" it goes unread, and the patch reads as
    /// built right up until the viewer stays dark.
    pub broken: Vec<(String, String)>,
    pub error: Option<String>,
    pub status: String,
}
impl Default for Assistant {
    fn default() -> Self {
        let keys = Keys::load();
        let cli_found: BTreeMap<Provider, bool> = Provider::ALL
            .iter()
            .filter(|p| !p.needs_key())
            .map(|p| (*p, cli::binary(*p).is_some()))
            .collect();
        // Open on a provider that is already usable, so somebody with
        // OPENAI_API_KEY set — or with Claude Code installed and nothing
        // else — lands somewhere that works.
        let provider = Provider::ALL
            .iter()
            .copied()
            .find(|p| match p.needs_key() {
                true => keys.get(*p).is_some(),
                false => cli_found.get(p).copied().unwrap_or(false),
            })
            .unwrap_or(Provider::Anthropic);
        Assistant {
            open: false,
            bar: true,
            collapsed: false,
            focus_bar: false,
            provider,
            model: provider.default_model().to_string(),
            key_input: String::new(),
            prompt: String::new(),
            conversation: otd_ai::Conversation::default(),
            submitted_prompt: String::new(),
            allow_delete: false,
            scope_selection: false,
            scoped_to: Vec::new(),
            image: None,
            seen: otd_ai::Seen::Reference,
            watch: None,
            image_tex: None,
            bar_rect: egui::Rect::NOTHING,
            keys,
            cli_found,
            cli_version: BTreeMap::new(),
            pending: None,
            last: None,
            warnings: Vec::new(),
            broken: Vec::new(),
            error: None,
            status: String::new(),
        }
    }
}
impl Assistant {
    /// A new project must not receive an old request or old creative context.
    pub fn reset_context(&mut self) {
        self.conversation.clear();
        self.submitted_prompt.clear();
        self.pending = None;
    }

    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }

    /// Which providers are ready to use, for the picker. A key for the three
    /// that take one, an installed CLI for the two that do not.
    pub(crate) fn ready(&self, provider: Provider) -> bool {
        match provider.needs_key() {
            true => self.keys.get(provider).is_some(),
            false => self.cli_found.get(&provider).copied().unwrap_or(false),
        }
    }

    /// Nothing configured for the provider currently selected. A key typed
    /// but not yet saved counts — it works for this request.
    pub(crate) fn not_configured(&self) -> bool {
        !self.ready(self.provider)
            && (!self.provider.needs_key() || self.key_input.trim().is_empty())
    }

    /// Re-run the CLI lookup, for after somebody installs one without
    /// restarting the editor.
    pub(crate) fn recheck_cli(&mut self, provider: Provider) {
        self.cli_found
            .insert(provider, cli::binary(provider).is_some());
        self.cli_version.insert(provider, cli::detect(provider));
    }

    /// There is enough here to ask with: a prompt, a picture, or both.
    ///
    /// An image on its own is a request — "make this" — which is why this is
    /// not simply a check on the text box.
    pub fn has_request(&self) -> bool {
        !self.prompt.trim().is_empty() || self.image.is_some()
    }

    /// Load a file as the reference image, reporting failure where the user
    /// is already looking rather than swallowing it.
    ///
    /// Decoding and shrinking a large still takes a moment; it happens here,
    /// once, rather than on the worker at send time, so the size shown in the
    /// bar is the size that will actually be sent.
    pub fn attach(&mut self, path: &std::path::Path) {
        match otd_ai::Image::load(path) {
            Ok(image) => {
                self.image = Some(image);
                self.seen = otd_ai::Seen::Reference;
                self.image_tex = None;
                self.error = None;
            }
            Err(e) => self.error = Some(e),
        }
    }

    pub fn detach(&mut self) {
        self.image = None;
        self.seen = otd_ai::Seen::Reference;
        self.image_tex = None;
    }

    /// The thumbnail as a texture, uploaded on first use.
    pub(crate) fn thumbnail(&mut self, ctx: &egui::Context) -> Option<egui::TextureHandle> {
        let image = self.image.as_ref()?;
        if self.image_tex.is_none() {
            let thumb = &image.thumb;
            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                [thumb.width as usize, thumb.height as usize],
                &thumb.rgba,
            );
            self.image_tex =
                Some(ctx.load_texture("assistant-reference", pixels, egui::TextureOptions::LINEAR));
        }
        self.image_tex.clone()
    }
}
