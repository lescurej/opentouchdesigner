use egui::{Color32, RichText};
use otd_core::Value;

use crate::app::OtdApp;

pub(crate) fn script_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let mut source = app.graph.node(id).params["source"].value.as_str();
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Source");
        ui.label(
            RichText::new("Python — set `rows` for Script DAT; `def onFrameStart` for Execute DATs")
                .weak()
                .small(),
        );
    });
    let editor_id = ui.id().with(("script", id));
    let me = app.graph.path(id);
    let label = format!("<{}>", me);
    let now = ui.input(|i| i.time);
    let paths = crate::code_lint::operator_paths(&app.graph);
    let response = {
        let rt = app.engine.runtime().lock().expect("engine runtime lock");
        let engines = &rt.engines;
        app.code_lint.touch(editor_id, &source, now);
        app.code_lint
            .poll(editor_id, now, engines, &app.graph, &me, &label);
        let issues = app.code_lint.issues(editor_id);
        crate::code_editor::show(
            ui,
            &mut source,
            crate::code_editor::CodeEditorConfig {
                language: "python",
                rows: 16,
                id: editor_id,
                interactive: true,
            },
            Some(crate::code_editor::CodeEditorExtras {
                issues,
                op_paths: &paths,
                me_path: &me,
                graph: &app.graph,
                engines,
            }),
        )
    };
    if response.changed() {
        app.edit(&format!("script:{id:?}"));
        let _ = app.graph.set_param(id, "source", Value::Str(source));
    }
    crate::code_lint::show_issues(ui, app.code_lint.issues(editor_id));
    if let Some(msg) = app.engine_rt().engines.node_message(&app.graph, id) {
        ui.colored_label(
            crate::canvas::node_message::text_color(msg.level),
            RichText::new(&msg.text).small().monospace(),
        );
    }
    ui.add_space(4.0);
}
pub(crate) fn shader_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let mut source = app.graph.node(id).params["source"].value.as_str();
    let is_glsl = app
        .graph
        .node(id)
        .params
        .get("language")
        .map(|p| p.value.as_str() == "glsl")
        .unwrap_or(false);

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Source");
        ui.label(
            RichText::new(if is_glsl {
                "Shadertoy GLSL — write mainImage(); iTime, iResolution, iFrame are provided"
            } else {
                "WGSL fragment body — in.uv, U.time.x, U.res, sample0/1(uv), U.p0..p3"
            })
            .weak()
            .small(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button("Import ISF…")
                .on_hover_text(
                    "Load an ISF shader (.fs). Its inputs become parameters on this node.",
                )
                .clicked()
            {
                app.import_isf(id);
            }
        });
    });

    let lang = if is_glsl { "glsl" } else { "rs" };
    let editor_id = ui.id().with(("shader", id));
    let response = crate::code_editor::show(
        ui,
        &mut source,
        crate::code_editor::CodeEditorConfig {
            language: lang,
            rows: 16,
            id: editor_id,
            interactive: true,
        },
        None,
    );
    if response.changed() {
        app.edit(&format!("shader:{id:?}"));
        let _ = app.graph.set_param(id, "source", Value::Str(source));
    }

    // The compile error comes from the GPU engine, not the parameter, because
    // the shader is only compiled when the node cooks.
    match app.engine_rt().engines.top.shader_error(id) {
        Some(err) => {
            ui.colored_label(
                Color32::from_rgb(235, 120, 120),
                RichText::new(err).small().monospace(),
            );
            ui.label(
                RichText::new("holding the last shader that compiled")
                    .weak()
                    .small(),
            );
        }
        None => {
            ui.colored_label(
                Color32::from_rgb(130, 200, 140),
                RichText::new("compiled").small(),
            );
        }
    }
    ui.add_space(4.0);
}
