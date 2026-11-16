use otd_core::NodeId;

use crate::app::OtdApp;

pub(crate) enum SelectionFlag {
    Viewer,
    Display,
    CloneImmune,
    ViewerActive,
    Lock,
    Bypass,
}

fn selection_targets(app: &OtdApp) -> Vec<NodeId> {
    if !app.canvas.selection.is_empty() {
        app.canvas.selection
            .iter()
            .copied()
            .filter(|id| app.graph.contains(*id))
            .collect()
    } else if let Some(id) = app.canvas.selected.filter(|id| app.graph.contains(*id)) {
        vec![id]
    } else {
        Vec::new()
    }
}

pub(crate) fn toggle_flag_on_selection(app: &mut OtdApp, flag: SelectionFlag) {
    let Some(anchor) = app
        .canvas.selected
        .filter(|id| app.graph.contains(*id))
        .or_else(|| app.canvas.selection.iter().next().copied())
    else {
        return;
    };
    toggle_flag_at(app, flag, anchor);
}

pub(crate) fn toggle_flag_at(app: &mut OtdApp, flag: SelectionFlag, anchor: NodeId) {
    if !app.graph.contains(anchor) {
        return;
    }
    if matches!(flag, SelectionFlag::Display) {
        let fam = app.graph.node(anchor).family;
        if !crate::canvas::flags::display_flag_allowed(fam) {
            app.document.status = if fam == otd_core::Family::Comp {
                "Display (D) is not used on COMP — use Viewer (V)".into()
            } else {
                "Display (D) is not used on MAT".into()
            };
            return;
        }
    }
    let ids = if app.is_selected(anchor) && app.canvas.selection.len() > 1 {
        selection_targets(app)
    } else {
        vec![anchor]
    };
    if ids.is_empty() {
        return;
    }
    let primary = if ids.contains(&anchor) {
        anchor
    } else {
        ids[0]
    };
    let node = app.graph.node(primary);
    let next = match flag {
        SelectionFlag::Viewer => !node.flags.viewer,
        SelectionFlag::Display => !node.flags.bg_display,
        SelectionFlag::CloneImmune => !node.flags.clone_immune,
        SelectionFlag::ViewerActive => !node.flags.viewer_active,
        SelectionFlag::Lock => !node.flags.lock,
        SelectionFlag::Bypass => !node.flags.bypass,
    };
    let tag = match flag {
        SelectionFlag::Viewer => "viewer flag",
        SelectionFlag::Display => "bg display",
        SelectionFlag::CloneImmune => "clone immune",
        SelectionFlag::ViewerActive => "viewer active",
        SelectionFlag::Lock => "lock",
        SelectionFlag::Bypass => "bypass",
    };
    app.edit(tag);
    let mut toggled = 0usize;
    for id in ids {
        if matches!(flag, SelectionFlag::Display)
            && !crate::canvas::flags::display_flag_allowed(app.graph.node(id).family)
        {
            continue;
        }
        if app.graph.node(id).flags.lock && !matches!(flag, SelectionFlag::Lock) {
            continue;
        }
        let f = &mut app.graph.node_mut(id).flags;
        match flag {
            SelectionFlag::Viewer => f.viewer = next,
            SelectionFlag::Display => f.bg_display = next,
            SelectionFlag::CloneImmune => f.clone_immune = next,
            SelectionFlag::ViewerActive => {
                f.viewer_active = next;
                if next && !f.viewer {
                    f.viewer = true;
                }
            }
            SelectionFlag::Lock => f.lock = next,
            SelectionFlag::Bypass => f.bypass = next,
        }
        toggled += 1;
    }
    app.document.history.end_gesture();
    if toggled > 0 && matches!(flag, SelectionFlag::Display) {
        let state = if next { "on" } else { "off" };
        app.document.status = format!("Display {state} on {toggled} operator(s)");
    } else if toggled == 0 && matches!(flag, SelectionFlag::Display) {
        app.document.status = "Display (D) is not available on COMP or MAT".into();
    } else if toggled > 0 && matches!(flag, SelectionFlag::ViewerActive) {
        let state = if next { "on" } else { "off" };
        app.document.status = format!("Viewer Active {state} on {toggled} operator(s)");
    }
}
