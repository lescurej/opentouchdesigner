
use egui::RichText;
use otd_ai::recipes::{self, Needs, Recipe};
use otd_ai::patch;
use otd_core::{Family, NodeId};

use crate::app::OtdApp;

use super::patch::{check_shader, land};

pub fn apply_recipe(app: &mut OtdApp, recipe: &'static Recipe) {
    app.assistant.error = None;
    app.assistant.warnings.clear();
    app.assistant.broken.clear();
    app.assistant.last = None;
    app.assistant.status.clear();

    let mut plan = match recipe.plan(&app.registry) {
        Ok(plan) => plan,
        Err(e) => {
            app.assistant.error = Some(format!("recipe {}: {e}", recipe.name));
            return;
        }
    };

    // A video recipe goes on whatever TOP is selected. With nothing selected
    // it still has to build something you can see, so a generator stands in
    // and the message says how to swap it for a clip.
    let mut aside = String::new();
    if recipe.needs == Needs::Video {
        // The selection, or — nothing selected — the one camera or clip on
        // the canvas. A recipe that ignored the only footage there is and
        // parked a noise generator beside it was how `source1` ended up
        // dangling in real patches.
        let source = app
            .canvas.selected
            .filter(|id| app.graph.contains(*id))
            .filter(|id| app.graph.node(*id).family == Family::Top)
            .or_else(|| {
                let sources: Vec<NodeId> = app
                    .graph
                    .children(app.canvas.current)
                    .iter()
                    .copied()
                    .filter(|id| {
                        let op = app.graph.node(*id).op_type.as_str();
                        op == otd_gpu::ops::VIDEO_DEVICE_IN || op == otd_gpu::ops::MOVIE_IN
                    })
                    .collect();
                match sources.as_slice() {
                    [one] => Some(*one),
                    _ => None,
                }
            })
            .map(|id| app.graph.node(id).name.clone());
        match source {
            Some(name) => recipes::with_source(&mut plan, &name),
            None => {
                recipes::stand_in_source(&mut plan);
                aside = format!(
                    " Noise is standing in for the clip: drop a movie onto {}, \
                     or select your clip first next time.",
                    recipes::SOURCE
                );
            }
        }
    }

    // Land it in clear canvas, to the right of everything already there and
    // level with the selection, rather than on top of the last patch.
    let children = app.graph.children(app.canvas.current);
    let right = children
        .iter()
        .map(|id| app.graph.node(*id).pos[0])
        .fold(None, |m: Option<f32>, x| Some(m.map_or(x, |m| m.max(x))));
    let level = app
        .canvas.selected
        .filter(|id| app.graph.contains(*id))
        .map(|id| app.graph.node(id).pos[1])
        .or_else(|| children.first().map(|id| app.graph.node(*id).pos[1]));
    if let (Some((lo, _)), Some(right)) = (plan.bounds(), right) {
        let dx = right + crate::canvas::NODE_W + 80.0 - lo[0];
        let dy = level.unwrap_or(lo[1]) - lo[1];
        plan.offset(dx, dy);
    }

    let broken = patch::shader_problems(recipe.value(), check_shader);
    if land(app, &plan, broken) {
        app.assistant.conversation.record(
            &format!("Applied recipe {}: {}", recipe.name, recipe.prompt),
            &format!("{}{}", plan.notes, aside),
        );
        app.assistant.status = format!("{} · {}", recipe.name, app.assistant.status);
        if let Some(last) = &mut app.assistant.last {
            last.push_str(&aside);
        }
    }
}
pub(crate) fn recipes_menu(app: &mut OtdApp, ui: &mut egui::Ui) {
    ui.menu_button("Recipes", |ui| {
        for group in recipes::groups() {
            ui.menu_button(group, |ui| {
                for recipe in recipes::all().iter().filter(|r| r.group == group) {
                    let row = ui.button(&recipe.name).on_hover_text(&recipe.prompt);
                    if row.clicked() {
                        apply_recipe(app, recipe);
                        ui.close();
                    } else if row.secondary_clicked() {
                        app.assistant.prompt = recipe.prompt.clone();
                        ui.close();
                    }
                }
            });
        }
        ui.separator();
        ui.label(
            RichText::new(
                "Click: built here, no model needed. Right-click: its request, in the box to edit.",
            )
            .weak()
            .small(),
        );
    })
    .response
    .on_hover_text("Ready-made patches — the same ones the assistant learns from");
}
