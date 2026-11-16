use super::material::{camera, light, material_of};
use super::types::{DrawItem, SceneDescription};
use crate::math::{self};
use crate::scene::{self, Scene};
use otd_core::{EvalContext, Graph, Node};

pub fn describe(
    graph: &Graph,
    render_node: &Node,
    ctx: &EvalContext,
    scene: &dyn Scene,
    aspect: f32,
) -> SceneDescription {
    let geometry_root = scene::s(render_node, ctx, "geometry");
    let items = collect_items(graph, geometry_root.trim(), ctx, scene);

    let camera_path = scene::s(render_node, ctx, "camera");
    let (view, camera_pos, projection) = camera(graph, camera_path.trim(), ctx, aspect);

    let (light_dir, light_color) = light(graph, scene::s(render_node, ctx, "light").trim(), ctx);

    SceneDescription {
        items,
        view_proj: math::mul(projection, view),
        camera_pos,
        // The view matrix's basis rows are the camera's axes in world space;
        // it is column-major, so they read down the columns.
        camera_right: [view[0][0], view[1][0], view[2][0]],
        camera_up: [view[0][1], view[1][1], view[2][1]],
        light_dir,
        light_color,
        background: scene::v4(render_node, ctx, "background"),
        ambient: scene::f(render_node, ctx, "ambient"),
        depth: scene::s(render_node, ctx, "output") == "depth",
        depth_near: scene::f(render_node, ctx, "depthnear"),
        depth_far: scene::f(render_node, ctx, "depthfar"),
        cull: match scene::menu(render_node, ctx, "cull") {
            1 => Some(wgpu::Face::Front),
            2 => None,
            _ => Some(wgpu::Face::Back),
        },
        wireframe: scene::b(render_node, ctx, "wireframe"),
    }
}

/// Every Geometry component under `root` — or the whole project when the
/// Render TOP names nothing, which is the useful default while patching.
pub(crate) fn collect_items(graph: &Graph, root: &str, ctx: &EvalContext, scene: &dyn Scene) -> Vec<DrawItem> {
    let start = if root.is_empty() {
        Some(graph.root())
    } else {
        graph.find(root)
    };
    let Some(start) = start else {
        return Vec::new();
    };

    let mut geometries = Vec::new();
    let mut stack = vec![start];
    while let Some(id) = stack.pop() {
        let Some(node) = graph.get(id) else { continue };
        if node.op_type == scene::GEOMETRY {
            geometries.push(id);
            // A Geometry component's children are its own business.
            continue;
        }
        stack.extend(node.children.iter().copied());
    }
    // Stable order, so a two-object scene does not flicker between frames.
    geometries.sort_by_key(|id| graph.path(*id));

    geometries
        .into_iter()
        .filter_map(|id| {
            let node = graph.get(id)?;
            let sop_path = scene::s(node, ctx, "sop");
            let geometry = scene.geometry(sop_path.trim())?.clone();
            if geometry.is_empty() {
                return None;
            }
            let surface = material_of(graph, &scene::s(node, ctx, "material"), ctx);
            let geometry = if surface.shading == scene::Shading::PointSprite {
                billboards(&geometry)
            } else {
                geometry
            };
            Some(DrawItem {
                geometry,
                model: math::trs(
                    scene::v3(node, ctx, "translate"),
                    scene::v3(node, ctx, "rotate"),
                    scene::v3(node, ctx, "scale"),
                ),
                instances: scene::instances(node, ctx, scene),
                base_color: surface.base_color,
                material: surface.material,
                color_map: surface.color_map,
                shading: surface.shading,
                wireframe: surface.wireframe,
            })
        })
        .collect()
}

/// One quad per point, ready for the vertex shader to face at the camera.
///
/// Expanded on the CPU rather than in a geometry shader, which WebGPU does not
/// have, and rather than by instancing, which is already spoken for by the
/// Geometry COMP. The corner is carried in `uv` as 0..1 — which is also
/// exactly what a sprite texture wants, so it costs no extra attribute.
pub(crate) fn billboards(geometry: &otd_sop::Geometry) -> otd_sop::Geometry {
    let mut points = Vec::with_capacity(geometry.points.len() * 4);
    let mut indices = Vec::with_capacity(geometry.points.len() * 6);
    for p in &geometry.points {
        let base = points.len() as u32;
        for uv in [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]] {
            points.push(otd_sop::Point { uv, ..*p });
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    otd_sop::Geometry {
        points,
        indices,
        topology: otd_sop::Topology::Triangles,
        mat_path: geometry.mat_path.clone(),
    }
}

/// The three edges of every triangle, as index pairs for a line list.
///
/// Deduplicated, so a closed mesh does not draw each shared edge twice — at a
/// few thousand triangles that is half the work for an identical picture.
pub fn triangle_edges(geometry: &otd_sop::Geometry) -> Vec<u32> {
    let source: Vec<u32> = if geometry.indices.is_empty() {
        (0..geometry.num_points() as u32).collect()
    } else {
        geometry.indices.clone()
    };
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(source.len() * 2);
    for tri in source.chunks(3) {
        if tri.len() < 3 {
            continue;
        }
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let key = if a < b { (a, b) } else { (b, a) };
            if seen.insert(key) {
                out.extend([a, b]);
            }
        }
    }
    out
}
