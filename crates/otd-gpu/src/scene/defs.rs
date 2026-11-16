use otd_core::{Connector, Family, OpDef, OpRegistry};

use super::params::*;

/// How a material wants its surface shaded. Packed into the render uniform's
/// spare slot rather than compiled as separate pipelines: the branch is on a
/// uniform, so it is the same for every fragment in a draw and costs nothing
/// a real divergence would.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shading {
    /// Metallic/roughness, the default.
    Pbr = 0,
    /// No lighting at all — the colour is the colour. What you want for
    /// anything already lit, and for geometry standing in as a UI element.
    Constant = 1,
    /// Lambert plus an explicit Blinn highlight, dialled by shininess rather
    /// than derived from roughness.
    Phong = 2,
    /// Every point becomes a camera-facing quad. Unlit, like Constant: a
    /// sprite is a picture, not a surface.
    PointSprite = 3,
}

pub const GEOMETRY: &str = "geometryCOMP";
pub const CAMERA: &str = "cameraCOMP";
pub const LIGHT: &str = "lightCOMP";
pub const PBR: &str = "pbrMAT";
pub const CONSTANT_MAT: &str = "constantMAT";
pub const PHONG: &str = "phongMAT";
pub const WIREFRAME: &str = "wireframeMAT";
pub const POINT_SPRITE: &str = "pointspriteMAT";

/// The scene components and materials.
///
/// Geometry, Camera and Light are components: they hold settings and are
/// referenced by a Render TOP, rather than producing anything themselves.
pub fn defs() -> Vec<OpDef> {
    vec![
        OpDef {
            type_name: GEOMETRY,
            input_families: &[],
            label: "Geometry",
            family: Family::Comp,
            inputs: &[],
            summary: "Places a SOP in the scene, optionally instanced.",
            time_dependent: false,
            params: params_geometry,
            connector: Connector::None,
        },
        OpDef {
            type_name: CAMERA,
            input_families: &[],
            label: "Camera",
            family: Family::Comp,
            inputs: &[],
            summary: "A viewpoint for a Render TOP.",
            time_dependent: false,
            params: params_camera,
            connector: Connector::None,
        },
        OpDef {
            type_name: LIGHT,
            input_families: &[],
            label: "Light",
            family: Family::Comp,
            inputs: &[],
            summary: "A directional light, aimed from its position at the origin.",
            time_dependent: false,
            params: params_light,
            connector: Connector::None,
        },
        OpDef {
            type_name: PBR,
            input_families: &[],
            label: "PBR",
            family: Family::Mat,
            inputs: &["color"],
            summary: "Base colour, metallic, roughness and emission, with an optional map.",
            time_dependent: false,
            params: params_pbr,
            connector: Connector::None,
        },
        OpDef {
            type_name: CONSTANT_MAT,
            input_families: &[],
            label: "Constant",
            family: Family::Mat,
            inputs: &["color"],
            summary: "Flat colour, unaffected by lights.",
            time_dependent: false,
            params: params_constant_mat,
            connector: Connector::None,
        },
        OpDef {
            type_name: PHONG,
            input_families: &[],
            label: "Phong",
            family: Family::Mat,
            inputs: &["color"],
            summary: "Diffuse and a Blinn highlight, dialled by shininess.",
            time_dependent: false,
            params: params_phong,
            connector: Connector::None,
        },
        OpDef {
            type_name: POINT_SPRITE,
            input_families: &[],
            label: "Point Sprite",
            family: Family::Mat,
            inputs: &["color"],
            summary: "Draws every point as a camera-facing quad.",
            time_dependent: false,
            params: params_point_sprite,
            connector: Connector::None,
        },
        OpDef {
            type_name: WIREFRAME,
            input_families: &[],
            label: "Wireframe",
            family: Family::Mat,
            inputs: &[],
            summary: "Draws the edges rather than the faces.",
            time_dependent: false,
            params: params_wireframe,
            connector: Connector::None,
        },
    ]
}

pub fn register_all(registry: &mut OpRegistry) {
    for def in defs() {
        registry.register(def);
    }
}
