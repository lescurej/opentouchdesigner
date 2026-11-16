use otd_core::indexmap::IndexMap;
use otd_core::Param;

// -------------------------------------------------------- operator table

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

pub(crate) fn params_geometry() -> IndexMap<String, Param> {
    params! {
        "sop" => Param::str("").with_label("SOP").as_path_ref(),
        "material" => Param::str("").with_label("Material").as_path_ref(),
        "translate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Translate"),
        "rotate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Rotate"),
        "scale" => Param::xyz([1.0, 1.0, 1.0]).with_label("Scale"),
        "instancing" => Param::bool(false).with_label("Instancing"),
        "instancechop" => Param::str("").with_label("Instance CHOP").as_path_ref(),
        "instancetop" => Param::str("").with_label("Instance TOP").as_path_ref(),
        "instancescale" => Param::float(1.0).with_label("Instance Scale").with_range(0.0, 10.0),
        "instancecount" => Param::int(1024).with_label("Instance Count (TOP)").with_range(1.0, 1000000.0),
        "tx" => Param::str("tx").with_label("Translate X Channel"),
        "ty" => Param::str("ty").with_label("Translate Y Channel"),
        "tz" => Param::str("tz").with_label("Translate Z Channel"),
        "sx" => Param::str("").with_label("Scale X Channel"),
        "sy" => Param::str("").with_label("Scale Y Channel"),
        "sz" => Param::str("").with_label("Scale Z Channel"),
        "rx" => Param::str("").with_label("Rotate X Channel"),
        "ry" => Param::str("").with_label("Rotate Y Channel"),
        "rz" => Param::str("").with_label("Rotate Z Channel"),
        "cr" => Param::str("").with_label("Red Channel"),
        "cg" => Param::str("").with_label("Green Channel"),
        "cb" => Param::str("").with_label("Blue Channel"),
        "ca" => Param::str("").with_label("Alpha Channel"),
    }
}

pub(crate) fn params_camera() -> IndexMap<String, Param> {
    params! {
        "translate" => Param::xyz([0.0, 0.0, 5.0]).with_label("Translate"),
        "rotate" => Param::xyz([0.0, 0.0, 0.0]).with_label("Rotate"),
        "lookat" => Param::str("").with_label("Look At").as_path_ref(),
        "projection" => Param::menu("perspective", &["perspective", "orthographic"])
            .with_label("Projection"),
        "fov" => Param::float(45.0).with_label("Field of View").with_range(5.0, 170.0),
        "orthosize" => Param::float(4.0).with_label("Ortho Height").with_range(0.1, 100.0),
        "near" => Param::float(0.1).with_label("Near").with_range(0.001, 10.0),
        "far" => Param::float(200.0).with_label("Far").with_range(1.0, 10000.0),
    }
}

pub(crate) fn params_light() -> IndexMap<String, Param> {
    params! {
        "translate" => Param::xyz([3.0, 5.0, 4.0]).with_label("Translate"),
        "color" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color"),
        "intensity" => Param::float(1.0).with_label("Intensity").with_range(0.0, 8.0),
    }
}

pub(crate) fn params_pbr() -> IndexMap<String, Param> {
    params! {
        "basecolor" => Param::rgba([0.8, 0.8, 0.85, 1.0]).with_label("Base Color"),
        "metallic" => Param::float(0.0).with_label("Metallic").with_range(0.0, 1.0),
        "roughness" => Param::float(0.4).with_label("Roughness").with_range(0.02, 1.0),
        "emit" => Param::float(0.0).with_label("Emit").with_range(0.0, 4.0),
    }
}

pub(crate) fn params_constant_mat() -> IndexMap<String, Param> {
    params! {
        "basecolor" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color"),
        "emit" => Param::float(1.0).with_label("Brightness").with_range(0.0, 4.0),
    }
}

pub(crate) fn params_phong() -> IndexMap<String, Param> {
    params! {
        "basecolor" => Param::rgba([0.8, 0.8, 0.85, 1.0]).with_label("Diffuse"),
        "specular" => Param::float(0.5).with_label("Specular").with_range(0.0, 4.0),
        "shininess" => Param::float(32.0).with_label("Shininess").with_range(1.0, 256.0),
        "emit" => Param::float(0.0).with_label("Emit").with_range(0.0, 4.0),
    }
}

pub(crate) fn params_point_sprite() -> IndexMap<String, Param> {
    params! {
        "basecolor" => Param::rgba([1.0, 1.0, 1.0, 1.0]).with_label("Color"),
        "size" => Param::float(0.1).with_label("Size (world units)").with_range(0.001, 4.0),
        "emit" => Param::float(1.0).with_label("Brightness").with_range(0.0, 4.0),
        "round" => Param::bool(true).with_label("Round"),
    }
}

pub(crate) fn params_wireframe() -> IndexMap<String, Param> {
    params! {
        "basecolor" => Param::rgba([0.6, 0.9, 1.0, 1.0]).with_label("Color"),
        "emit" => Param::float(1.0).with_label("Brightness").with_range(0.0, 4.0),
    }
}
