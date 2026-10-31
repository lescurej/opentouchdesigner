/// A one-input, one-shader, no-surprises TOP.
macro_rules! filter_top {
    ($ty:expr, $label:expr, $summary:expr, $inputs:expr, $shader:expr, $sizing:expr, $params:path, $pack:path $(,)?) => {
        $crate::ops::TopSpec {
            def: otd_core::OpDef {
                type_name: $ty,
                input_families: &[],
                label: $label,
                family: otd_core::Family::Top,
                inputs: $inputs,
                summary: $summary,
                time_dependent: false,
                params: $params,
                connector: otd_core::Connector::None,
            },
            shader: $shader,
            sizing: $sizing,
            two_pass: false,
            dynamic_shader: false,
            pack: $pack,
        }
    };
}

/// The named blend operators, which differ only in the operation they pin.
macro_rules! blend_top {
    ($ty:expr, $label:expr, $summary:expr, $pack:path $(,)?) => {
        filter_top!(
            $ty,
            $label,
            $summary,
            &["in1", "in2"],
            include_str!("../shaders/composite.wgsl"),
            $crate::ops::Sizing::Input0,
            $crate::ops::blend::params_blend,
            $pack,
        )
    };
}
