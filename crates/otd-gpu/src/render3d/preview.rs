use super::types::{DrawItem, SceneDescription};
use crate::math::{self};
use crate::scene::{self};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    /// Multiplier on the geometry bounding radius.
    pub distance: f32,
    pub pan: [f32; 3],
}

const SOP_PREVIEW_FOV_DEG: f32 = 45.0;
const SOP_PREVIEW_FIT_MARGIN: f32 = 1.18;

impl Default for OrbitCamera {
    fn default() -> Self {
        OrbitCamera {
            yaw: 0.85,
            pitch: 0.45,
            distance: 1.0,
            pan: [0.0; 3],
        }
    }
}

impl OrbitCamera {
    /// Default tumble angles with zoom reset — pair with auto-fit distance in
    /// [`describe_sop_preview`].
    pub fn reset_view() -> Self {
        Self::default()
    }
}

/// Cache key for a SOP preview render.
pub fn sop_preview_cache_key(
    geo_stamp: u64,
    orbit: &OrbitCamera,
    wireframe: bool,
    orthographic: bool,
    show_axes: bool,
) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    geo_stamp.hash(&mut h);
    orbit.yaw.to_bits().hash(&mut h);
    orbit.pitch.to_bits().hash(&mut h);
    orbit.distance.to_bits().hash(&mut h);
    for v in orbit.pan {
        v.to_bits().hash(&mut h);
    }
    wireframe.hash(&mut h);
    orthographic.hash(&mut h);
    show_axes.hash(&mut h);
    h.finish()
}

fn framing(geo: &otd_sop::Geometry) -> ([f32; 3], f32) {
    let (lo, hi) = geo.bounds();
    let center = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let ext = [
        (hi[0] - lo[0]).max(1e-4),
        (hi[1] - lo[1]).max(1e-4),
        (hi[2] - lo[2]).max(1e-4),
    ];
    // Half the AABB diagonal — conservative for any orbit angle.
    let radius = 0.5 * (ext[0] * ext[0] + ext[1] * ext[1] + ext[2] * ext[2]).sqrt();
    (center, radius.max(1e-4))
}

/// Mesh bounds merged with the world axes (origin → +X/+Y/+Z) for camera framing.
fn preview_framing(geo: &otd_sop::Geometry, include_axes: bool) -> ([f32; 3], f32, f32) {
    let (_, mesh_radius) = framing(geo);
    let axis_span = axis_display_span(mesh_radius);
    if !include_axes {
        let (center, radius) = framing(geo);
        return (center, radius, axis_span);
    }
    let (mut lo, mut hi) = geo.bounds();
    lo[0] = lo[0].min(0.0);
    lo[1] = lo[1].min(0.0);
    lo[2] = lo[2].min(0.0);
    hi[0] = hi[0].max(axis_span);
    hi[1] = hi[1].max(axis_span);
    hi[2] = hi[2].max(axis_span);
    let center = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let ext = [
        (hi[0] - lo[0]).max(1e-4),
        (hi[1] - lo[1]).max(1e-4),
        (hi[2] - lo[2]).max(1e-4),
    ];
    let radius = 0.5 * (ext[0] * ext[0] + ext[1] * ext[1] + ext[2] * ext[2]).sqrt();
    (center, radius.max(1e-4), axis_span)
}

/// Camera distance so a bounding sphere fits in a perspective frustum.
pub(crate) fn fit_distance(bounding_radius: f32, aspect: f32, fov_deg: f32) -> f32 {
    let r = bounding_radius.max(1e-4);
    let aspect = aspect.max(1e-3);
    let v_half = (fov_deg * 0.5).to_radians();
    let d_vert = r / v_half.sin();
    let h_half = (v_half.tan() * aspect).atan();
    let d_horiz = r / h_half.sin();
    SOP_PREVIEW_FIT_MARGIN * d_vert.max(d_horiz)
}

fn orbit_eye(
    center: [f32; 3],
    radius: f32,
    aspect: f32,
    orbit: &OrbitCamera,
) -> ([f32; 3], f32) {
    let pitch = orbit.pitch.clamp(-1.4, 1.4);
    let base = fit_distance(radius, aspect, SOP_PREVIEW_FOV_DEG);
    let d = (base * orbit.distance.max(0.35)).max(0.05);
    let cp = pitch.cos();
    let target = [
        center[0] + orbit.pan[0],
        center[1] + orbit.pan[1],
        center[2] + orbit.pan[2],
    ];
    let eye = [
        target[0] + d * cp * orbit.yaw.sin(),
        target[1] + d * pitch.sin(),
        target[2] + d * cp * orbit.yaw.cos(),
    ];
    (eye, d)
}

fn axis_display_span(radius: f32) -> f32 {
    (radius * 1.25).ceil().max(1.0)
}

/// World-origin RGB axes for the SOP viewer (X red, Y green, Z blue).
fn axis_helper_geometry(span: f32) -> otd_sop::Geometry {
    use otd_sop::{Point, Topology};
    let s = span.max(1e-3);
    let pt = |x: f32, y: f32, z: f32, color: [f32; 4]| Point {
        position: [x, y, z],
        normal: [0.0, 1.0, 0.0],
        uv: [0.0, 0.0],
        color,
    };
    let points = vec![
        pt(0.0, 0.0, 0.0, [1.0, 0.35, 0.35, 1.0]),
        pt(s, 0.0, 0.0, [1.0, 0.35, 0.35, 1.0]),
        pt(0.0, 0.0, 0.0, [0.35, 1.0, 0.4, 1.0]),
        pt(0.0, s, 0.0, [0.35, 1.0, 0.4, 1.0]),
        pt(0.0, 0.0, 0.0, [0.4, 0.55, 1.0, 1.0]),
        pt(0.0, 0.0, s, [0.4, 0.55, 1.0, 1.0]),
    ];
    otd_sop::Geometry {
        points,
        indices: vec![0, 1, 2, 3, 4, 5],
        topology: Topology::Lines,
        ..Default::default()
    }
}

/// Build a one-mesh scene for the SOP network viewer.
pub fn describe_sop_preview(
    geo: &otd_sop::Geometry,
    aspect: f32,
    orbit: &OrbitCamera,
    wireframe: bool,
    orthographic: bool,
    show_axes: bool,
) -> SceneDescription {
    let (center, radius, axis_span) = preview_framing(geo, show_axes);
    let aspect = aspect.max(1e-3);
    let (eye, cam_dist) = orbit_eye(center, radius, aspect, orbit);
    let target = [
        center[0] + orbit.pan[0],
        center[1] + orbit.pan[1],
        center[2] + orbit.pan[2],
    ];
    let view = math::look_at(eye, target, [0.0, 1.0, 0.0]);
    let near = (radius * 0.02).max(1e-3);
    let far = (cam_dist + radius * 4.0 + 10.0).max(near + 1.0);
    let zoom = orbit.distance.max(0.35);
    let projection = if orthographic {
        let height = radius * 2.0 * SOP_PREVIEW_FIT_MARGIN * zoom;
        math::orthographic(height, aspect, near, far)
    } else {
        math::perspective(SOP_PREVIEW_FOV_DEG, aspect, near, far)
    };

    let item = DrawItem {
        geometry: geo.clone(),
        model: math::IDENTITY,
        instances: vec![scene::Instance::default()],
        base_color: [1.0, 1.0, 1.0, 1.0],
        material: [0.35, 48.0, 0.0, 0.0],
        color_map: None,
        shading: scene::Shading::Phong,
        wireframe: false,
    };

    let mut items = vec![item];
    if show_axes {
        items.push(DrawItem {
            geometry: axis_helper_geometry(axis_span),
            model: math::IDENTITY,
            instances: vec![scene::Instance::default()],
            base_color: [1.0, 1.0, 1.0, 1.0],
            // Constant MAT: material.z is emit — must be 1 or lines render black.
            material: [0.0, 0.0, 1.0, 0.0],
            color_map: None,
            shading: scene::Shading::Constant,
            wireframe: false,
        });
    }

    SceneDescription {
        items,
        view_proj: math::mul(projection, view),
        camera_pos: eye,
        camera_right: [view[0][0], view[1][0], view[2][0]],
        camera_up: [view[0][1], view[1][1], view[2][1]],
        light_dir: [0.35, 0.75, 0.55, 1.0],
        light_color: [1.0, 1.0, 1.0, 1.0],
        background: [0.11, 0.12, 0.14, 1.0],
        ambient: 0.22,
        depth: false,
        depth_near: near,
        depth_far: far,
        cull: Some(wgpu::Face::Back),
        wireframe,
    }
}
