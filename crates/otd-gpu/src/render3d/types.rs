use crate::math::Mat4;
use crate::scene::{self, Instance};
use otd_core::NodeId;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct SceneUniforms {
    pub(crate) view_proj: Mat4,
    pub(crate) model: Mat4,
    pub(crate) camera: [f32; 4],
    /// The camera's right and up axes in world space. A billboard cannot be
    /// built without them, and deriving them in the shader would mean
    /// inverting the view matrix per vertex.
    pub(crate) camera_right: [f32; 4],
    pub(crate) camera_up: [f32; 4],
    pub(crate) light_dir: [f32; 4],
    pub(crate) light_color: [f32; 4],
    pub(crate) base_color: [f32; 4],
    pub(crate) material: [f32; 4],
    pub(crate) render: [f32; 4],
    pub(crate) depth_range: [f32; 4],
}

/// One thing to draw: geometry, where it is, what it looks like, and how many
/// copies.
pub struct DrawItem {
    pub geometry: otd_sop::Geometry,
    pub model: Mat4,
    pub instances: Vec<Instance>,
    pub base_color: [f32; 4],
    /// metallic, roughness, emit, use_texture — reinterpreted per shading
    /// model; see `material_of`.
    pub material: [f32; 4],
    pub color_map: Option<NodeId>,
    pub shading: scene::Shading,
    /// Draw this item's edges rather than its faces.
    pub wireframe: bool,
}

/// Everything the pass needs, gathered from the graph before any GPU work.
pub struct SceneDescription {
    pub items: Vec<DrawItem>,
    pub view_proj: Mat4,
    pub camera_pos: [f32; 3],
    pub camera_right: [f32; 3],
    pub camera_up: [f32; 3],
    pub light_dir: [f32; 4],
    pub light_color: [f32; 4],
    pub background: [f32; 4],
    pub ambient: f32,
    /// Render distance-from-camera as grey instead of shading the scene.
    pub depth: bool,
    pub depth_near: f32,
    pub depth_far: f32,
    pub cull: Option<wgpu::Face>,
    pub wireframe: bool,
}
