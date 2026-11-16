use std::sync::mpsc::TryRecvError;

use otd_ai::{Ask, patch};
use otd_core::NodeId;

use crate::app::OtdApp;

pub(crate) fn check_shader(source: &str, is_glsl: bool) -> Result<(), String> {
    if is_glsl {
        otd_gpu::shader::validate_glsl(&otd_gpu::shader::wrap_glsl(source))
    } else {
        otd_gpu::shader::validate_wgsl(&otd_gpu::shader::wrap_wgsl(source))
    }
}
pub(crate) fn local_command(app: &mut OtdApp) -> bool {
    let line = app.assistant.prompt.trim().to_string();
    if !line.starts_with('/') {
        return false;
    }
    let word = line.split_whitespace().next().unwrap_or("").to_lowercase();
    match word.as_str() {
        "/clear" => {
            let n = app.clear_network();
            app.assistant.prompt.clear();
            // Both, because the two halves of the assistant show different
            // things: the panel has a status line and the collapsed bar does
            // not, and a command that appears to do nothing gets typed twice.
            let said = match n {
                0 => "The network was already empty.".to_string(),
                1 => "Cleared 1 node. Cmd/Ctrl+Z puts it back.".to_string(),
                n => format!("Cleared {n} nodes. Cmd/Ctrl+Z puts them back."),
            };
            app.assistant.status = said.clone();
            app.assistant.last = Some(said);
        }
        "/watch" => {
            // Whatever follows the command is direction for the effect.
            app.assistant.prompt = line["/watch".len()..].trim().to_string();
            crate::watch::toggle(app);
        }
        "/help" => {
            app.assistant.prompt.clear();
            app.assistant.last = Some(
                "/clear empties the network you are looking at (one undo).\n\
                 /watch [direction] turns the camera on and builds an effect for what it sees.\n\
                 /help is this.\n\
                 Anything not starting with a slash is a request for the model."
                    .into(),
            );
        }
        other => {
            app.assistant.error = Some(format!("no command {other} — try /clear or /help"));
        }
    }
    true
}
pub(crate) fn send(app: &mut OtdApp) {
    app.assistant.error = None;
    app.assistant.warnings.clear();
    app.assistant.broken.clear();
    app.assistant.last = None;
    app.assistant.status.clear();

    // Commands are answered here and never reach a provider.
    if local_command(app) {
        return;
    }

    // A key typed but not saved should still work for this one request.
    let mut keys = app.assistant.keys.clone();
    if !app.assistant.key_input.trim().is_empty() {
        keys.set(
            app.assistant.provider,
            otd_ai::Key::new(app.assistant.key_input.clone()),
        );
    }
    // A CLI provider has no key to be missing; `cli::complete` reports an
    // absent or signed-out binary itself, in its own words.
    let key = match app.assistant.provider.env_var() {
        Some(var) => match keys.get(app.assistant.provider).cloned() {
            Some(key) => key,
            None => {
                app.assistant.error = Some(format!(
                    "no API key for {} — paste one, or set {var}",
                    app.assistant.provider.label(),
                ));
                return;
            }
        },
        None => otd_ai::Key::new(""),
    };

    let scope = match app.assistant.scope_selection {
        true => scope_of(app),
        false => Vec::new(),
    };
    app.assistant.scoped_to = scope
        .iter()
        .map(|id| app.graph.node(*id).name.clone())
        .collect();

    let request = otd_ai::request_with_history(
        &Ask {
            provider: app.assistant.provider,
            model: app.assistant.model.clone(),
            prompt: app.assistant.prompt.clone(),
            image: app.assistant.image.clone(),
            seen: app.assistant.seen.clone(),
            graph: &app.graph,
            parent: app.canvas.current,
            selected: app.canvas.selected,
            viewer: app.canvas.viewer,
            registry: &app.registry,
            allow_delete: app.assistant.allow_delete,
            scope: &scope,
        },
        &app.assistant.conversation,
    );
    app.assistant.submitted_prompt = if app.assistant.prompt.trim().is_empty() {
        match app.assistant.seen {
            otd_ai::Seen::Reference => "Build from the attached reference image",
            otd_ai::Seen::Camera => "Make the live camera look striking",
            otd_ai::Seen::Result(_) => "Fix what the result was measured doing wrong",
        }
        .into()
    } else {
        app.assistant.prompt.clone()
    };

    let (tx, rx) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("otd-assistant".into())
        .spawn(move || {
            let result = otd_ai::complete_with_repair(&request, &key, &keys, Some(check_shader));
            let _ = tx.send(result.map(|r| (r.text, r.repaired)));
        });
    app.assistant.pending = Some(rx);
}
pub(crate) fn scope_of(app: &OtdApp) -> Vec<NodeId> {
    if !app.canvas.selection.is_empty() {
        return app
            .canvas.selection
            .iter()
            .copied()
            .filter(|id| app.graph.contains(*id))
            .collect();
    }
    app.canvas.selected
        .filter(|id| app.graph.contains(*id))
        .into_iter()
        .collect()
}
pub(crate) fn gate_deletes(plan: &mut otd_ai::Plan, allow: bool) -> Option<String> {
    if allow || plan.deletes.is_empty() {
        return None;
    }
    let kept = std::mem::take(&mut plan.deletes).join(", ");
    Some(format!(
        "kept {kept} — turn on 🗑 to let the assistant remove operators"
    ))
}
pub(crate) fn poll(app: &mut OtdApp) {
    let Some(rx) = &app.assistant.pending else {
        return;
    };
    let reply = match rx.try_recv() {
        Ok(reply) => reply,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => {
            app.assistant.pending = None;
            app.assistant.error = Some("the request thread stopped unexpectedly".into());
            return;
        }
    };
    app.assistant.pending = None;

    let (text, repaired) = match reply {
        Ok(pair) => pair,
        Err(e) => {
            app.assistant.error = Some(e);
            return;
        }
    };
    let mut plan = match otd_ai::plan_from_reply(&text, &app.registry) {
        Ok(plan) => plan,
        Err(e) => {
            app.assistant.error = Some(e);
            return;
        }
    };
    // The prompt asked it not to; this is what makes it true. A rule in a
    // prompt is a request, and the one thing the user cannot undo by looking
    // at the canvas is an operator that is no longer on it.
    if let Some(kept) = gate_deletes(&mut plan, app.assistant.allow_delete) {
        app.assistant.warnings.push(kept);
    }
    // Same reasoning one line up: the prompt asked, this is what makes it
    // true. Filtered against the selection as it was when the request went
    // out, not as it is now.
    let scoped_to = std::mem::take(&mut app.assistant.scoped_to);
    if let Some(skipped) = patch::gate_scope(&mut plan, &scoped_to) {
        app.assistant.warnings.push(skipped);
    }

    // A shader that survived the repair round still broken would otherwise
    // be a red node and a silently black output.
    let broken = patch::extract_json(&text)
        .map(|json| patch::shader_problems(&json, check_shader))
        .unwrap_or_default();
    if land(app, &plan, broken) {
        let outcome = format!(
            "{} Warnings: {:?}. Shader errors: {:?}",
            plan.notes, app.assistant.warnings, app.assistant.broken
        );
        app.assistant
            .conversation
            .record(&app.assistant.submitted_prompt, &outcome);
        if let Some(repair) = repaired {
            app.assistant
                .status
                .push_str(&format!(" · {}", repair.label()));
        }
    }
}
pub(crate) fn land(app: &mut OtdApp, plan: &patch::Plan, broken: Vec<(String, String)>) -> bool {
    // One checkpoint, before anything is created, so the whole patch undoes
    // as the single thing it was.
    app.edit("assistant");
    let current = app.canvas.current;
    match patch::apply(&mut app.graph, current, &app.registry, plan) {
        Ok((applied, viewer)) => {
            if let Some(viewer) = viewer {
                if app.graph.node(viewer).family == otd_core::Family::Comp {
                    app.set_perform_ui(viewer);
                }
            }
            // A plan may now remove nodes, so ids held outside the graph can
            // be stale the moment it lands. The cook takes a removed node as
            // a root and panics on the lookup, so this is not cosmetic.
            app.canvas.selection.retain(|id| app.graph.contains(*id));
            app.canvas.selected = applied
                .created
                .first()
                .and_then(|name| app.graph.find_from(current, name))
                .or_else(|| app.canvas.selected.filter(|id| app.graph.contains(*id)));
            if let Some(id) = app.canvas.selected {
                app.select_only(id);
            }
            if !app
                .canvas.viewer
                .map(|id| {
                    app.graph.contains(id)
                        && app.graph.node(id).family == otd_core::Family::Comp
                })
                .unwrap_or(false)
            {
                app.canvas.viewer = None;
            }

            let mut parts = Vec::new();
            if !applied.created.is_empty() {
                parts.push(format!("{} node(s)", applied.created.len()));
            }
            if applied.wired > 0 {
                parts.push(format!("{} wire(s)", applied.wired));
            }
            if !applied.changed.is_empty() {
                parts.push(format!("{} retuned", applied.changed.len()));
            }
            if !applied.removed.is_empty() {
                parts.push(format!("{} removed", applied.removed.len()));
            }
            if parts.is_empty() {
                parts.push("nothing changed".into());
            }
            app.assistant.status = parts.join(", ");
            app.assistant.warnings = applied.warnings;
            // Anything the model built and then forgot to join on. The nodes
            // are real and they cook; saying so beats leaving them to be
            // found.
            let loose = patch::dangling(plan);
            if !loose.is_empty() {
                app.assistant.warnings.push(format!(
                    "created but wired to nothing: {}",
                    loose.join(", ")
                ));
            }
            app.assistant.broken = broken;
            app.assistant.last = Some(if plan.notes.trim().is_empty() {
                "Built.".to_string()
            } else {
                plan.notes.clone()
            });
            true
        }
        Err(e) => {
            app.assistant.error = Some(e);
            false
        }
    }
}
