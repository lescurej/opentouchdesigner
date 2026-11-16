use std::collections::HashMap;

use otd_core::{EvalContext, Graph, NodeId};

use otd_core::Family;

use crate::panel::{self, is_panel_surface_widget, CONTAINER};

fn param_f(node: &otd_core::Node, key: &str, ctx: &EvalContext, default: f32) -> f32 {
    node.param(key)
        .map(|p| p.eval(ctx).as_f64() as f32)
        .unwrap_or(default)
}

fn param_s(node: &otd_core::Node, key: &str, ctx: &EvalContext, default: &str) -> String {
    node.param(key)
        .map(|p| p.eval(ctx).as_str())
        .unwrap_or_else(|| default.to_string())
}

fn local_layout_rect(node: &otd_core::Node, ctx: &EvalContext) -> [f32; 4] {
    let hmode = param_s(node, "hmode", ctx, "fixed");
    let vmode = param_s(node, "vmode", ctx, "fixed");

    let mut x = param_f(node, "x", ctx, 0.05);
    let mut y = param_f(node, "y", ctx, 0.05);
    let mut w = param_f(node, "w", ctx, 0.2);
    let mut h = param_f(node, "h", ctx, 0.08);

    match hmode.as_str() {
        "fill" => {
            x = 0.0;
            w = 1.0;
        }
        "anchors" => {
            let la = param_f(node, "leftanchor", ctx, 0.0);
            let ra = param_f(node, "rightanchor", ctx, 1.0);
            let lo = param_f(node, "leftoffset", ctx, 0.0).clamp(-0.5, 0.5);
            let ro = param_f(node, "rightoffset", ctx, 0.0).clamp(-0.5, 0.5);
            x = (la + lo).clamp(0.0, 1.0);
            w = (ra - la - lo + ro).max(0.01);
        }
        _ => {}
    }

    match vmode.as_str() {
        "fill" => {
            y = 0.0;
            h = 1.0;
        }
        "anchors" => {
            let ba = param_f(node, "bottomanchor", ctx, 0.0);
            let ta = param_f(node, "topanchor", ctx, 1.0);
            let bo = param_f(node, "bottomoffset", ctx, 0.0).clamp(-0.5, 0.5);
            let to = param_f(node, "topoffset", ctx, 0.0).clamp(-0.5, 0.5);
            y = (ba + bo).clamp(0.0, 1.0);
            h = (ta - ba - bo + to).max(0.01);
        }
        _ => {}
    }

    match param_s(node, "fixedaspect", ctx, "off").as_str() {
        "horizontal" => {
            let aspect = param_f(node, "aspect", ctx, 1.0).max(0.01);
            h = (w / aspect).min(1.0);
        }
        "vertical" => {
            let aspect = param_f(node, "aspect", ctx, 1.0).max(0.01);
            w = (h * aspect).min(1.0);
        }
        _ => {}
    }

    x += param_f(node, "postoffsetx", ctx, 0.0);
    y += param_f(node, "postoffsety", ctx, 0.0);

    [x, y, w.max(0.001), h.max(0.001)]
}

pub fn participates_in_layout(graph: &Graph, id: NodeId) -> bool {
    let Some(node) = graph.get(id) else {
        return false;
    };
    if node.op_type == CONTAINER {
        return true;
    }
    if node
        .parent
        .and_then(|p| graph.get(p))
        .is_some_and(|parent| parent.op_type == CONTAINER && node.family == Family::Comp)
    {
        return true;
    }
    graph
        .get(id)
        .is_some_and(|n| panel::is_panel(&n.op_type) && is_panel_surface_widget(graph, id))
}

fn child_respects_parent_align(graph: &Graph, kid: NodeId, ctx: &EvalContext) -> bool {
    let allow = graph
        .node(kid)
        .param("alignallow")
        .map(|p| p.eval(ctx).as_str())
        .unwrap_or_else(|| "allow".to_string());
    allow == "allow"
}

pub fn apply_container_layout(graph: &Graph, rects: &mut HashMap<NodeId, [f32; 4]>) {
    let ctx = EvalContext::default();
    let containers: Vec<NodeId> = graph
        .walk()
        .into_iter()
        .filter(|id| graph.get(*id).is_some_and(|n| n.op_type == CONTAINER))
        .collect();

    for cid in containers {
        let align = param_s(graph.node(cid), "align", &ctx, "none");
        if align == "none" || align == "nodes" {
            continue;
        }
        let Some(mut inner) = rects.get(&cid).copied() else {
            continue;
        };
        let margin = |key: &str| param_f(graph.node(cid), key, &ctx, 0.0);
        inner[0] += margin("marginl") * inner[2];
        inner[1] += margin("margint") * inner[3];
        inner[2] -= (margin("marginl") + margin("marginr")) * inner[2];
        inner[3] -= (margin("margint") + margin("marginb")) * inner[3];

        let spacing = param_f(graph.node(cid), "spacing", &ctx, 0.02);
        let alignmax = param_f(graph.node(cid), "alignmax", &ctx, 0.0) as usize;

        let mut kids: Vec<NodeId> = graph
            .node(cid)
            .children
            .iter()
            .copied()
            .filter(|id| participates_in_layout(graph, *id))
            .filter(|id| child_respects_parent_align(graph, *id, &ctx))
            .collect();
        kids.sort_by(|a, b| {
            let order = |id: NodeId| {
                graph
                    .node(id)
                    .param("alignorder")
                    .map(|p| p.eval(&ctx).as_f64())
                    .unwrap_or(0.0)
            };
            order(*a)
                .partial_cmp(&order(*b))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| graph.path(*a).cmp(&graph.path(*b)))
        });
        if kids.is_empty() {
            continue;
        }

        match align.as_str() {
            "horizlr" | "horizrl" => layout_row(
                &mut inner,
                spacing,
                align == "horizrl",
                alignmax,
                &kids,
                rects,
            ),
            "verttb" | "vertbt" => layout_col(
                &mut inner,
                spacing,
                align == "vertbt",
                alignmax,
                &kids,
                rects,
            ),
            "gridrows" => layout_grid(&mut inner, spacing, true, &kids, rects),
            "gridcols" => layout_grid(&mut inner, spacing, false, &kids, rects),
            _ => {}
        }
    }
}

fn layout_row(
    inner: &mut [f32; 4],
    spacing: f32,
    reverse: bool,
    max_per_line: usize,
    kids: &[NodeId],
    rects: &mut HashMap<NodeId, [f32; 4]>,
) {
    let max_per_line = max_per_line.max(1);
    let mut order: Vec<NodeId> = kids.to_vec();
    if reverse {
        order.reverse();
    }
    let gap = spacing * inner[2];
    let lines: Vec<&[NodeId]> = order.chunks(max_per_line).collect();
    let line_h = inner[3] / lines.len().max(1) as f32;
    for (line_i, line) in lines.iter().enumerate() {
        let n = line.len() as f32;
        let cell_w = (inner[2] - gap * (n - 1.0).max(0.0)) / n.max(1.0);
        let y = inner[1] + line_i as f32 * line_h;
        for (i, id) in line.iter().enumerate() {
            let local = rects.get(id).copied().unwrap_or([0.0, 0.0, 0.2, 0.08]);
            let x = inner[0] + i as f32 * (cell_w + gap);
            rects.insert(*id, [x, y, cell_w, line_h * local[3].max(0.05)]);
        }
    }
}

fn layout_col(
    inner: &mut [f32; 4],
    spacing: f32,
    reverse: bool,
    max_per_line: usize,
    kids: &[NodeId],
    rects: &mut HashMap<NodeId, [f32; 4]>,
) {
    let max_per_line = max_per_line.max(1);
    let mut order: Vec<NodeId> = kids.to_vec();
    if reverse {
        order.reverse();
    }
    let gap = spacing * inner[3];
    let cols: Vec<&[NodeId]> = order.chunks(max_per_line).collect();
    let col_w = inner[2] / cols.len().max(1) as f32;
    for (col_i, col) in cols.iter().enumerate() {
        let n = col.len() as f32;
        let cell_h = (inner[3] - gap * (n - 1.0).max(0.0)) / n.max(1.0);
        let x = inner[0] + col_i as f32 * col_w;
        for (i, id) in col.iter().enumerate() {
            let local = rects.get(id).copied().unwrap_or([0.0, 0.0, 0.2, 0.08]);
            let y = inner[1] + i as f32 * (cell_h + gap);
            rects.insert(*id, [x, y, col_w * local[2].max(0.05), cell_h]);
        }
    }
}

fn layout_grid(
    inner: &mut [f32; 4],
    spacing: f32,
    _rows: bool,
    kids: &[NodeId],
    rects: &mut HashMap<NodeId, [f32; 4]>,
) {
    let n = kids.len();
    let cols = ((n as f32).sqrt().ceil() as usize).max(1);
    let rows_n = n.div_ceil(cols);
    let gap_x = spacing * inner[2];
    let gap_y = spacing * inner[3];
    let cell_w = (inner[2] - gap_x * (cols.saturating_sub(1) as f32)) / cols as f32;
    let cell_h = (inner[3] - gap_y * (rows_n.saturating_sub(1) as f32)) / rows_n as f32;
    for (i, id) in kids.iter().enumerate() {
        let col = i % cols;
        let row = i / cols;
        rects.insert(
            *id,
            [
                inner[0] + col as f32 * (cell_w + gap_x),
                inner[1] + row as f32 * (cell_h + gap_y),
                cell_w,
                cell_h,
            ],
        );
    }
}

pub fn resolve_panel_rects(graph: &Graph) -> HashMap<NodeId, [f32; 4]> {
    let ctx = EvalContext::default();
    let mut rects = HashMap::new();

    fn resolve(
        graph: &Graph,
        id: NodeId,
        ctx: &EvalContext,
        rects: &mut HashMap<NodeId, [f32; 4]>,
    ) {
        if rects.contains_key(&id) {
            return;
        }
        let node = graph.node(id);
        let local = local_layout_rect(node, ctx);

        let rect = if let Some(parent) = node.parent.filter(|p| participates_in_layout(graph, *p)) {
            resolve(graph, parent, ctx, rects);
            let pr = rects[&parent];
            [
                pr[0] + local[0] * pr[2],
                pr[1] + local[1] * pr[3],
                local[2] * pr[2],
                local[3] * pr[3],
            ]
        } else {
            local
        };
        rects.insert(id, rect);
    }

    for id in graph.walk() {
        if participates_in_layout(graph, id) {
            resolve(graph, id, &ctx, &mut rects);
        }
    }
    apply_container_layout(graph, &mut rects);
    rects
}
