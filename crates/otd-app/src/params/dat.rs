use egui::RichText;
use otd_core::Value;

use crate::app::OtdApp;

pub(crate) fn show_dat_text_editor(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: otd_core::NodeId,
    rows: usize,
    text_area_id: egui::Id,
    interactive: bool,
) -> egui::Response {
    let mut text = app.graph.node(id).params["text"].value.as_str();
    let lint_id = crate::code_editor::dat_editor_id(id);
    let me = crate::code_lint::me_path_for_node(&app.graph, id);
    let label = format!("<{}>", app.graph.path(id));
    let is_python = dat_text_editor_is_python(app, id);
    let now = ui.input(|i| i.time);
    let paths = crate::code_lint::operator_paths(&app.graph);
    let issues_owned = app.code_lint.issues(lint_id).to_vec();
    let language = if is_python { "python" } else { "text" };
    let response = {
        let rt = app.engine.runtime().lock().expect("engine runtime lock");
        let engines = &rt.engines;
        if is_python {
            app.code_lint.touch(lint_id, &text, now);
            app.engine
                .poll_code_lint(&mut app.code_lint, lint_id, now, &app.graph, &me, &label);
        }
        let extras = is_python.then(|| crate::code_editor::CodeEditorExtras {
            issues: &issues_owned,
            op_paths: &paths,
            me_path: &me,
            graph: &app.graph,
            engines,
        });
        crate::code_editor::show(
        ui,
        &mut text,
        crate::code_editor::CodeEditorConfig {
            language,
            rows,
            id: text_area_id,
            interactive,
        },
        extras,
    )
    };
    if response.changed() && interactive {
        app.edit(&format!("text:{id:?}"));
        let _ = app.graph.set_param(id, "text", Value::Str(text));
    }
    response
}
pub(crate) fn dat_text_editor_is_python(app: &OtdApp, id: otd_core::NodeId) -> bool {
    if crate::code_editor::is_callback_dat_text(app, id) {
        return true;
    }
    app.graph
        .node(id)
        .params
        .get("text")
        .is_some_and(|p| p.is_script())
}
pub(crate) fn dat_uses_text_param_editor(app: &OtdApp, id: otd_core::NodeId) -> bool {
    app.graph.node(id).params.contains_key("text")
        && app.engine_rt().engines.dat_data(id)
            .is_some_and(|d| d.is_text)
}
pub(crate) fn dat_node_viewer_editor(
    app: &mut OtdApp,
    ui: &mut egui::Ui,
    id: otd_core::NodeId,
    body: egui::Rect,
    zoom: f32,
    interactive: bool,
) {
    if zoom < 0.32 || body.width() < 48.0 || body.height() < 28.0 {
        return;
    }
    let body = if app.canvas.canvas_rect.is_positive() {
        body.intersect(app.canvas.canvas_rect)
    } else {
        body
    };
    if body.width() < 48.0 || body.height() < 28.0 {
        return;
    }
    let row_h = (14.0 * zoom).clamp(11.0, 18.0);
    let rows = ((body.height() / row_h).floor() as usize).clamp(4, 64);
    ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
        ui.set_clip_rect(body);
        ui.set_width(body.width());
        show_dat_text_editor(
            app,
            ui,
            id,
            rows,
            crate::code_editor::dat_node_viewer_editor_id(id),
            interactive,
        );
    });
}
pub(crate) fn text_editor(app: &mut OtdApp, ui: &mut egui::Ui, id: otd_core::NodeId) {
    let rows = app
        .engine_rt()
        .engines
        .dat_data(id)
        .map(|d| format!("{} × {}", d.num_rows(), d.num_cols()))
        .unwrap_or_default();

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.strong("Contents");
        ui.label(RichText::new(rows).weak().small());
    });

    let is_python = dat_text_editor_is_python(app, id);
    let _ = show_dat_text_editor(
        app,
        ui,
        id,
        if is_python { 18 } else { 14 },
        crate::code_editor::dat_editor_id(id),
        true,
    );
    if is_python {
        ui.label(
            RichText::new("Autocomplete in op('…') and ch('…', '…') as you type")
                .weak()
                .small(),
        );
        let editor_id = crate::code_editor::dat_editor_id(id);
        crate::code_lint::show_issues(ui, app.code_lint.issues(editor_id));
    }
    ui.add_space(4.0);
}
