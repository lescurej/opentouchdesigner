use super::*;

use super::*;
    use otd_core::Node;
    use otd_core::{CookContext, CookEngine};
    use otd_engine::registry;
    use otd_sop::SopHost;

    fn wired_source(_graph: &Graph, node: &Node, input_index: usize) -> Option<NodeId> {
        node.inputs.get(input_index).copied().flatten()
    }

    #[test]
    fn protomarquee_structure_filein_uses_data_sidecar() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protomarquee.tox.dir");
        if !dir.is_dir() {
            return;
        }
        let files = crate::load_dir(&dir).expect("load");
        let reg = registry();
        let mut graph = Graph::new();
        let root = graph.root();
        import_into_graph(&mut graph, root, &files, &dir, &reg, "protomarquee").expect("import");
        let structure = graph.find("/protomarquee/structure").expect("structure");
        let file = graph.node(structure).params["file"].value.as_str();
        assert!(
            file.contains("structure.data"),
            "file param should use .data sidecar, got {file}"
        );
        if Path::new(&file).is_relative() {
            assert!(
                dir.join(&file).is_file(),
                "relative sidecar missing: {file}"
            );
        } else {
            assert!(
                Path::new(&file).is_file(),
                "absolute sidecar missing: {file}"
            );
        }
    }

    #[test]
    fn teapot_tox_lists_network_files() {
        let tox = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = crate::decode_with_fallback(&tox).expect("decode teapot.tox");
        let mut nodes: Vec<String> = decode
            .files
            .keys()
            .filter(|k| k.ends_with(".n") && k.contains("base1/"))
            .cloned()
            .collect();
        nodes.sort();
        let data: Vec<String> = decode
            .files
            .keys()
            .filter(|k| k.ends_with(".data") && k.contains("base1/"))
            .cloned()
            .collect();
        eprintln!("teapot nodes: {nodes:?}");
        eprintln!("teapot data sidecars: {data:?}");
        assert!(!nodes.is_empty(), "teapot should contain SOP nodes");
        assert!(
            nodes.iter().any(|n| n.contains("box")),
            "expected a box SOP in tox, got {nodes:?}"
        );
        assert!(
            nodes
                .iter()
                .any(|n| n.contains("filein") || n.contains("triangle")),
            "expected a filein or triangle SOP in tox, got {nodes:?}"
        );
        assert!(
            nodes.iter().any(|n| n.contains("grid1")),
            "expected grid1 SOP in tox, got {nodes:?}"
        );
    }

    #[test]
    fn teapot_filein_cooks_when_present() {
        let tox = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = crate::decode_with_fallback(&tox).expect("decode");
        let dir = crate::expansion_filesystem_root(&tox, &decode).expect("materialize");
        let reg = registry();
        let mut graph = Graph::new();
        let root = graph.root();
        import_into_graph(&mut graph, root, &decode.files, &dir, &reg, "base1").expect("import");
        let filein1 = graph
            .find("/base1/filein1")
            .expect("filein1 (built-in .tog / teapot calibration)");
        let mut host = SopHost::new();
        let mut cook = CookEngine::new();
        cook.cook_frame(&graph, &[filein1], &CookContext::default(), &mut host)
            .expect("cook");
        let geo = host.geometry(filein1).expect("geometry");
        assert_eq!(geo.num_points(), 1760);
        assert_eq!(
            geo.indices.len() / 3,
            3368,
            "1670 quads + 28 tris from the GPD polygon run"
        );
        let (lo, hi) = geo.bounds();
        assert!(
            (hi[0] - lo[0] - 3.2).abs() < 0.05 && (hi[2] - lo[2] - 3.2).abs() < 0.05,
            "teapot XZ extent {lo:?}..{hi:?}"
        );
        assert!(
            (hi[1] - lo[1] - 0.916).abs() < 0.02,
            "teapot height {lo:?}..{hi:?}"
        );
        let p = geo.points[0].position;
        assert!((p[0] + 0.690644).abs() < 1e-4, "point 0 X {p:?}");
        assert!((p[1] - 0.225069).abs() < 1e-4, "point 0 Y {p:?}");
        assert!(p[2].abs() < 1e-5, "point 0 Z {p:?}");
        let n = geo.points[0].normal;
        assert!((n[0] + 0.883494).abs() < 1e-4, "point 0 N {n:?}");
        assert!((n[1] - 0.468443).abs() < 1e-4, "point 0 N {n:?}");
    }

    #[test]
    fn teapot_triangle_cooks_from_locked_sidecar_when_present() {
        let tox = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = crate::decode_with_fallback(&tox).expect("decode");
        let dir = crate::expansion_filesystem_root(&tox, &decode).expect("materialize");
        let reg = registry();
        let mut graph = Graph::new();
        let root = graph.root();
        import_into_graph(&mut graph, root, &decode.files, &dir, &reg, "base1").expect("import");
        let tri = graph
            .find("/base1/triangle")
            .expect("triangle file-in calibration");
        let file = graph.node(tri).params["file"].value.as_str();
        assert!(
            file.ends_with(".data"),
            "locked tox should cook from embedded sidecar, got: {file}"
        );
        let mut host = SopHost::new();
        let mut cook = CookEngine::new();
        cook.cook_frame(&graph, &[tri], &CookContext::default(), &mut host)
            .expect("cook");
        let geo = host.geometry(tri).expect("geometry");
        assert_eq!(geo.num_points(), 3);
        assert_eq!(geo.indices.len() / 3, 1);
        let (lo, hi) = geo.bounds();
        assert!(
            hi[0] - lo[0] > 0.5 && hi[1] - lo[1] > 0.5,
            "embedded triangle.data positions not decoded yet (expected ~2×3); bounds {lo:?}..{hi:?}"
        );
    }

    #[test]
    fn teapot_grid_cooks_from_locked_sidecar_when_present() {
        let tox = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = crate::decode_with_fallback(&tox).expect("decode");
        let dir = crate::expansion_filesystem_root(&tox, &decode).expect("materialize");
        let reg = registry();
        let mut graph = Graph::new();
        let root = graph.root();
        import_into_graph(&mut graph, root, &decode.files, &dir, &reg, "base1").expect("import");
        let grid = graph.find("/base1/grid1").expect("grid1 calibration");
        assert!(
            graph.node(grid).flags.lock,
            "grid1 should import with lock on (cooks from grid1.data)"
        );
        assert!(
            decode.files.contains_key("base1/grid1.data"),
            "embedded grid1.data sidecar"
        );
        let mut host = SopHost::new();
        let mut cook = CookEngine::new();
        cook.cook_frame(&graph, &[grid], &CookContext::default(), &mut host)
            .expect("cook");
        let geo = host.geometry(grid).expect("geometry");
        assert_eq!(geo.num_points(), 4);
        assert_eq!(geo.indices.len() / 3, 2);
        let (lo, hi) = geo.bounds();
        assert!(
            (hi[0] - lo[0] - 1.0).abs() < 0.05 && (hi[1] - lo[1] - 1.0).abs() < 0.05,
            "2×2 grid bounds {lo:?}..{hi:?}"
        );
        assert!(
            hi[2].abs() < 0.05 && lo[2].abs() < 0.05,
            "grid in Z=0 plane"
        );
    }

    #[test]
    fn teapot_box_cooks_from_locked_sidecar_when_present() {
        let tox = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = crate::decode_with_fallback(&tox).expect("decode");
        let dir = crate::expansion_filesystem_root(&tox, &decode).expect("materialize");
        let reg = registry();
        let mut graph = Graph::new();
        let root = graph.root();
        import_into_graph(&mut graph, root, &decode.files, &dir, &reg, "base1").expect("import");
        let box_sop = graph.find("/base1/box").expect("box calibration");
        let file = graph.node(box_sop).params["file"].value.as_str();
        assert!(
            file.ends_with(".data"),
            "locked box should cook from embedded sidecar, got: {file}"
        );
        let mut host = SopHost::new();
        let mut cook = CookEngine::new();
        cook.cook_frame(&graph, &[box_sop], &CookContext::default(), &mut host)
            .expect("cook");
        let geo = host.geometry(box_sop).expect("geometry");
        assert!(geo.num_points() >= 24, "locked box points");
        let tris = geo.indices.len() / 3;
        if geo.num_points() >= 64 {
            assert!(
                (100..=120).contains(&tris),
                "subdivided box surface, got {tris} tris"
            );
        } else {
            assert_eq!(tris, 12, "unit box");
        }
        let (lo, hi) = geo.bounds();
        assert!(
            (hi[0] - lo[0] - 1.0).abs() < 0.08,
            "box width ~1: {lo:?}..{hi:?}"
        );
    }

    #[test]
    fn protomarquee_structure_filein_cooks_from_sidecar() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protomarquee.tox.dir");
        if !dir.is_dir() {
            return;
        }
        let files = crate::load_dir(&dir).expect("load");
        let reg = registry();
        let mut graph = Graph::new();
        let root = graph.root();
        import_into_graph(&mut graph, root, &files, &dir, &reg, "protomarquee").expect("import");
        let structure = graph.find("/protomarquee/structure").expect("structure");

        let mut host = SopHost::new();
        let mut cook = CookEngine::new();
        let ctx = CookContext::default();
        cook.cook_frame(&graph, &[structure], &ctx, &mut host)
            .expect("cook");
        let geo = host.geometry(structure).expect("geometry");
        let point_count = geo.num_points();
        let tri_count = geo.indices.len() / 3;
        assert!(
            point_count > 50,
            "structure.data should load points, got {point_count}"
        );
        assert!(tri_count > 50, "triangle count {tri_count}");

        let _ = graph.set_param(
            structure,
            "file",
            Value::Str("/nonexistent/structure.obj".into()),
        );
        cook.cook_frame(&graph, &[structure], &ctx, &mut host)
            .expect("cook fallback");
        let geo2 = host.geometry(structure).expect("geometry after fallback");
        assert_eq!(
            geo2.num_points(),
            point_count,
            "nested sidecar fallback should still resolve protomarquee/structure.data"
        );
    }

    #[test]
    fn protomarquee_chopexec_channels_and_chop() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protomarquee.tox.dir");
        if !dir.is_dir() {
            return;
        }
        let files = crate::load_dir(&dir).expect("load fixture");
        let reg = registry();
        let mut graph = Graph::new();
        let parent = graph.root();
        import_into_graph(&mut graph, parent, &files, &dir, &reg, "protomarquee").expect("import");
        let id = graph.find("/protomarquee/chopexec1").expect("chopexec1");
        assert_eq!(
            graph.node(id).params["channels"].value.as_str(),
            "Showlabel"
        );
        assert_eq!(graph.node(id).params["chop"].value.as_str(), "par1");
    }

    #[test]
    fn protomarquee_geotext_text_is_parent_name_expression() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protomarquee.tox.dir");
        if !dir.is_dir() {
            return;
        }
        let files = crate::load_dir(&dir).expect("load fixture");
        let reg = registry();
        let mut graph = Graph::new();
        let parent = graph.root();
        import_into_graph(&mut graph, parent, &files, &dir, &reg, "protomarquee").expect("import");
        let id = graph.find("/protomarquee/geotext1").expect("geotext1");
        assert_eq!(graph.node(id).op_type, "geotextCOMP");
        let text = &graph.node(id).params["text"];
        assert_eq!(text.mode, otd_core::param::ParamMode::Expression);
        assert_eq!(text.expression, "parent().name");
        assert!(
            graph.node(id).flags.render,
            "geotext should import with render on"
        );
    }

    #[test]
    fn protomarquee_math2_feeds_geo1_and_to_artnet() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protomarquee.tox.dir");
        if !dir.is_dir() {
            return;
        }
        let files = crate::load_dir(&dir).expect("load fixture");
        let reg = registry();
        let base = dir.clone();
        let mut graph = Graph::new();
        let parent = graph.root();
        import_into_graph(&mut graph, parent, &files, &base, &reg, "protomarquee").expect("import");

        let math2 = graph.find("/protomarquee/math2").expect("math2");
        let geo1 = graph.find("/protomarquee/geo1").expect("geo1");
        let artnet = graph.find("/protomarquee/toArtnet2").expect("toArtnet2");
        assert_eq!(
            wired_source(&graph, graph.node(geo1), 0),
            Some(math2),
            "geo1 in1 should be math2"
        );
        assert_eq!(
            wired_source(&graph, graph.node(artnet), 0),
            Some(math2),
            "toArtnet2 in1 should be math2"
        );
    }
