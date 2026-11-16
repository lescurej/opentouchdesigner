use crate::math::{self, Mat4};
use crate::scene::{self};
use otd_core::{EvalContext, Family, Graph, NodeId};

/// What a Geometry COMP's Material parameter resolves to.
pub struct Surface {
    pub base_color: [f32; 4],
    /// The four numbers the shader reads; what they mean depends on `shading`.
    pub material: [f32; 4],
    pub color_map: Option<NodeId>,
    pub shading: scene::Shading,
    /// Draw edges rather than faces.
    pub wireframe: bool,
}

impl Default for Surface {
    fn default() -> Self {
        Surface {
            base_color: [0.8, 0.8, 0.85, 1.0],
            material: [0.0, 0.4, 0.0, 0.0],
            color_map: None,
            shading: scene::Shading::Pbr,
            wireframe: false,
        }
    }
}

pub(crate) fn material_of(graph: &Graph, path: &str, ctx: &EvalContext) -> Surface {
    let path = path.trim();
    if path.is_empty() {
        return Surface::default();
    }
    let Some(node) = graph.find(path).and_then(|id| graph.get(id)) else {
        return Surface::default();
    };
    if node.family != Family::Mat {
        return Surface::default();
    }
    // A TOP wired into the material's input is its colour map.
    let map = node.inputs.first().copied().flatten();
    let has_map = if map.is_some() { 1.0 } else { 0.0 };
    let base_color = scene::v4(node, ctx, "basecolor");
    let emit = scene::f(node, ctx, "emit");

    match node.op_type.as_str() {
        scene::CONSTANT_MAT => Surface {
            base_color,
            // A Constant material's Brightness is the emit term, since emit is
            // the only one of the four that survives with the lighting off.
            material: [0.0, 0.0, emit, has_map],
            color_map: map,
            shading: scene::Shading::Constant,
            wireframe: false,
        },
        scene::WIREFRAME => Surface {
            base_color,
            material: [0.0, 0.0, emit, 0.0],
            color_map: None,
            shading: scene::Shading::Constant,
            wireframe: true,
        },
        scene::POINT_SPRITE => Surface {
            base_color,
            // x is the sprite's world size, y whether to round it off.
            material: [
                scene::f(node, ctx, "size"),
                if scene::b(node, ctx, "round") {
                    1.0
                } else {
                    0.0
                },
                emit,
                has_map,
            ],
            color_map: map,
            shading: scene::Shading::PointSprite,
            wireframe: false,
        },
        scene::PHONG => Surface {
            base_color,
            material: [
                scene::f(node, ctx, "specular"),
                scene::f(node, ctx, "shininess"),
                emit,
                has_map,
            ],
            color_map: map,
            shading: scene::Shading::Phong,
            wireframe: false,
        },
        _ => Surface {
            base_color,
            material: [
                scene::f(node, ctx, "metallic"),
                scene::f(node, ctx, "roughness"),
                emit,
                has_map,
            ],
            color_map: map,
            shading: scene::Shading::Pbr,
            wireframe: false,
        },
    }
}

/// The view and projection matrices for a camera, with a sensible default so
/// a Render TOP with no camera still shows something.
pub(crate) fn camera(graph: &Graph, path: &str, ctx: &EvalContext, aspect: f32) -> (Mat4, [f32; 3], Mat4) {
    let node = if path.is_empty() {
        None
    } else {
        graph.find(path).and_then(|id| graph.get(id))
    };
    let Some(node) = node.filter(|n| n.op_type == scene::CAMERA) else {
        let eye = [0.0, 0.0, 5.0];
        return (
            math::look_at(eye, [0.0; 3], [0.0, 1.0, 0.0]),
            eye,
            math::perspective(45.0, aspect, 0.1, 200.0),
        );
    };

    let translate = scene::v3(node, ctx, "translate");
    let rotate = scene::v3(node, ctx, "rotate");
    let look_at_path = scene::s(node, ctx, "lookat");

    // Aiming at another component beats dialling in Euler angles, and it is
    // what makes an orbiting camera one expression rather than three.
    let view = match graph
        .find(look_at_path.trim())
        .and_then(|id| graph.get(id))
        .filter(|_| !look_at_path.trim().is_empty())
    {
        Some(target) => math::look_at(
            translate,
            scene::v3(target, ctx, "translate"),
            [0.0, 1.0, 0.0],
        ),
        None => math::inverse_trs(translate, rotate, [1.0; 3]),
    };

    let near = scene::f(node, ctx, "near").max(1e-3);
    let far = scene::f(node, ctx, "far").max(near + 1e-3);
    let projection = if scene::menu(node, ctx, "projection") == 1 {
        math::orthographic(scene::f(node, ctx, "orthosize"), aspect, near, far)
    } else {
        math::perspective(scene::f(node, ctx, "fov"), aspect, near, far)
    };
    (view, translate, projection)
}

pub(crate) fn light(graph: &Graph, path: &str, ctx: &EvalContext) -> ([f32; 4], [f32; 4]) {
    let node = if path.is_empty() {
        None
    } else {
        graph.find(path).and_then(|id| graph.get(id))
    };
    let Some(node) = node.filter(|n| n.op_type == scene::LIGHT) else {
        // A key light over the viewer's shoulder, so an unlit scene is still
        // readable while patching.
        let d = normalize([0.4, 0.7, 0.6]);
        return ([d[0], d[1], d[2], 1.0], [1.0; 4]);
    };
    // The light points from its position at the origin, so moving it is the
    // whole interaction.
    let d = normalize(scene::v3(node, ctx, "translate"));
    (
        [d[0], d[1], d[2], scene::f(node, ctx, "intensity")],
        scene::v4(node, ctx, "color"),
    )
}

pub(crate) fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len < 1e-9 {
        [0.0, 1.0, 0.0]
    } else {
        [v[0] / len, v[1] / len, v[2] / len]
    }
}

// ------------------------------------------------------------- the pass
