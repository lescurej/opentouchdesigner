use super::helpers::no_params;
use super::operators::*;
use super::{IN, NULL, OUT, SopSpec, spec};
use otd_core::Connector;

pub(super) fn core_specs() -> Vec<SopSpec> {
    let mut v = vec![
            spec(
                "boxSOP",
                "Box",
                &[],
                "A box with flat-shaded faces.",
                params_box,
                cook_box,
            ),
            spec(
                "sphereSOP",
                "Sphere",
                &[],
                "A UV sphere.",
                params_sphere,
                cook_sphere,
            ),
            spec(
                "tubeSOP",
                "Tube",
                &[],
                "A cylinder, cone or tapered tube, with optional caps.",
                params_tube,
                cook_tube,
            ),
            spec(
                "torusSOP",
                "Torus",
                &[],
                "A torus.",
                params_torus,
                cook_torus,
            ),
            spec(
                "circleSOP",
                "Circle",
                &[],
                "A disc, ring or arc — filled, or a line to copy along.",
                params_circle,
                cook_circle,
            ),
            spec(
                "gridSOP",
                "Grid",
                &[],
                "A flat grid of quads — the usual thing to displace.",
                params_grid,
                cook_grid,
            ),
            spec(
                "lineSOP",
                "Line",
                &[],
                "A run of points between two positions.",
                params_line,
                cook_line,
            ),
            spec(
                "transformSOP",
                "Transform",
                &["in"],
                "Translate, rotate and scale points.",
                params_transform,
                cook_transform,
            ),
            spec(
                "noiseSOP",
                "Noise",
                &["in"],
                "Displace points by value noise, along their normals or freely.",
                params_noise,
                cook_noise,
            ),
            spec(
                "twistSOP",
                "Twist",
                &["in"],
                "Twist, bend or taper along one axis, growing with distance from the origin. \
                 Needs enough rows along that axis to curve smoothly.",
                params_twist,
                cook_twist,
            ),
            spec(
                "facetSOP",
                "Facet",
                &["in"],
                "Recompute normals from the triangles — put after Noise or Twist so lighting \
                 follows the new shape. `unique` gives flat, faceted shading.",
                params_facet,
                cook_facet,
            ),
            spec(
                "colorSOP",
                "Color",
                &["in"],
                "Set the colour carried by every point.",
                params_color,
                cook_color,
            ),
            spec(
                "mergeSOP",
                "Merge",
                &["a", "b"],
                "Combine two pieces of geometry.",
                no_params,
                cook_merge,
            ),
            spec(
                "blendSOP",
                "Blend",
                &["a", "b"],
                "Morph between two shapes by interpolating point positions.",
                params_blend,
                cook_blend,
            ),
            spec(
                "copySOP",
                "Copy",
                &["in"],
                "Stamp copies with a compounding transform.",
                params_copy,
                cook_copy,
            ),
            spec(
                NULL,
                "Null",
                &["in"],
                "Pass-through. A stable name to reference.",
                no_params,
                cook_null,
            ),
        ];
        let mut in_sop = spec(
            IN,
            "In",
            &[],
            "A geometry input on this component's node.",
            no_params,
            cook_null,
        );
        in_sop.def.connector = Connector::In;
        v.push(in_sop);
        let mut out_sop = spec(
            OUT,
            "Out",
            &["in"],
            "This component's geometry output.",
            no_params,
            cook_null,
        );
        out_sop.def.connector = Connector::Out;
        v.push(out_sop);
        v
}
