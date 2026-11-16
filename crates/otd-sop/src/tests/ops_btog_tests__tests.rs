use super::super::*;
    use super::super::geom::geometry_from;
    use super::super::header::P_CHAIN_SEED;
    use super::super::parse::read_detail;

    #[test]
    fn recognizes_a_version_5_header_only() {
        let mut buf = vec![0_u8; 100];
        buf[..8].copy_from_slice(MAGIC);
        assert!(!is_btog(&buf));
        assert!(parse(&buf).is_err());

        let empty = empty_btog(0);
        assert!(is_btog(&empty));
        let g = parse(&empty).expect("empty detail");
        assert_eq!(g.num_points(), 0);
    }

    #[test]
    fn parses_a_59_byte_single_point_export() {
        let raw = bytes_of(
            "42 74 6f 67 56 00 00 00 05 00 00 00 01 00 00 00 00 00 00 00 00 \
             00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 \
             bc 32 83 03 03 03 03 03 03 03 03 03 6a b2 83 03 00 ff",
        );
        assert_eq!(raw.len(), 59);
        let g = parse(&raw).expect("single point");
        assert_eq!(g.num_points(), 1);
        let p = g.points[0].position;
        assert!((p[0] + 1.0).abs() < 1e-6, "x {p:?}");
        assert!(p[1].abs() < 1e-6 && p[2].abs() < 1e-6, "{p:?}");
        assert!(parse(&raw[..raw.len() - 1]).is_err());
    }

    #[test]
    fn two_points_carry_the_plaintext_seed() {
        let x0 = f32::from_bits(0x3f80_0001);
        let mut buf = empty_btog(2);
        buf.truncate(buf.len() - 2);
        let (b0, seed) = encrypt_p(x0, 0.0, 0.0, 1.0, P_CHAIN_SEED);
        let (b1, _) = encrypt_p(5.0, 0.0, 0.0, 1.0, seed);
        buf.extend_from_slice(&b0);
        buf.extend_from_slice(&b1);
        buf.extend_from_slice(&[0x00, 0xff]);
        assert_eq!(buf.len(), 75);
        let g = parse(&buf).expect("two points");
        assert_eq!(g.num_points(), 2);
        assert!((g.points[0].position[0] - x0).abs() < 1e-6);
        assert!(
            (g.points[1].position[0] - 5.0).abs() < 1e-5,
            "{:?}",
            g.points[1].position
        );
    }

    #[test]
    fn point_attributes_set_the_record_stride() {
        let mut buf = Vec::new();
        push_header(&mut buf, 1, 0, 4, 0, 0, 0);
        push_attrib(&mut buf, "N", 3, 5, &[0.0, 0.0, 0.0]);
        push_attrib(&mut buf, "uv", 3, 0, &[0.0, 0.0, 0.0]);
        push_attrib(&mut buf, "Alpha", 1, 0, &[1.0]);
        push_attrib(&mut buf, "Cd", 3, 0, &[0.0, 0.0, 0.0]);
        let (block, _) = encrypt_p(1.0, 2.0, 3.0, 1.0, P_CHAIN_SEED);
        buf.extend_from_slice(&block);
        for v in [0.0f32, 1.0, 0.0, 0.25, 0.5, 0.0, 0.5, 0.1, 0.2, 0.3] {
            buf.extend_from_slice(&v.to_be_bytes());
        }
        buf.extend_from_slice(&[0x00, 0xff]);
        let detail = read_detail(&buf).expect("attribute file");
        assert_eq!(detail.point_start, 123);
        assert_eq!(detail.stride, 56);
        let g = geometry_from(detail).expect("geometry");
        let p = &g.points[0];
        assert!((p.position[0] - 1.0).abs() < 1e-5);
        assert!((p.position[1] - 2.0).abs() < 1e-5);
        assert!((p.position[2] - 3.0).abs() < 1e-5);
        assert!((p.normal[1] - 1.0).abs() < 1e-5);
        assert!((p.uv[0] - 0.25).abs() < 1e-5 && (p.uv[1] - 0.5).abs() < 1e-5);
        assert!((p.color[0] - 0.1).abs() < 1e-5);
        assert!((p.color[1] - 0.2).abs() < 1e-5);
        assert!((p.color[2] - 0.3).abs() < 1e-5);
        assert!((p.color[3] - 0.5).abs() < 1e-5);
    }

    fn empty_btog(npoints: u32) -> Vec<u8> {
        let mut buf = Vec::new();
        push_header(&mut buf, npoints, 0, 0, 0, 0, 0);
        buf.extend_from_slice(&[0x00, 0xff]);
        buf
    }

    fn push_header(
        buf: &mut Vec<u8>,
        npoints: u32,
        nprims: u32,
        npoint_attrib: u32,
        nvertex_attrib: u32,
        nprim_attrib: u32,
        ndetail: u32,
    ) {
        buf.extend_from_slice(b"Btog");
        buf.push(b'V');
        buf.extend_from_slice(&5u32.to_be_bytes());
        for v in [
            npoints,
            nprims,
            0,
            0,
            npoint_attrib,
            nvertex_attrib,
            nprim_attrib,
            ndetail,
        ] {
            buf.extend_from_slice(&v.to_be_bytes());
        }
    }

    fn push_attrib(buf: &mut Vec<u8>, name: &str, size: u16, type_code: u32, defaults: &[f32]) {
        buf.extend_from_slice(&(name.len() as u16).to_be_bytes());
        buf.extend_from_slice(name.as_bytes());
        buf.extend_from_slice(&size.to_be_bytes());
        buf.extend_from_slice(&type_code.to_be_bytes());
        for v in defaults {
            buf.extend_from_slice(&v.to_be_bytes());
        }
    }

    fn encrypt_p(x: f32, y: f32, z: f32, w: f32, seed: u8) -> ([u8; 16], u8) {
        let (wb, prev) = encrypt_f32(w, seed);
        let (zb, prev) = encrypt_f32(z, prev);
        let (yb, prev) = encrypt_f32(y, prev);
        let (xb, prev) = encrypt_f32(x, prev);
        let mut block = [0u8; 16];
        block[0..4].copy_from_slice(&xb);
        block[4..8].copy_from_slice(&yb);
        block[8..12].copy_from_slice(&zb);
        block[12..16].copy_from_slice(&wb);
        (block, prev)
    }

    fn encrypt_f32(value: f32, mut prev: u8) -> ([u8; 4], u8) {
        let plain = value.to_be_bytes();
        let mut out = [0u8; 4];
        for (slot, byte) in out.iter_mut().zip(plain) {
            *slot = byte ^ prev.wrapping_mul(17).wrapping_add(3);
            prev = byte;
        }
        (out, prev)
    }

    fn bytes_of(hex: &str) -> Vec<u8> {
        hex.split_whitespace()
            .map(|b| u8::from_str_radix(b, 16).expect("hex"))
            .collect()
    }

    #[test]
    fn loads_protomarquee_bulb_fixture_when_present() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bulb1_mesh.data");
        if !path.exists() {
            return;
        }
        let g = load(&path).expect("parse bulb");
        assert_eq!(g.num_points(), 96);
        assert!(
            g.indices.len() / 3 >= 25,
            "triangle count {}",
            g.indices.len() / 3
        );
        let (lo, hi) = g.bounds();
        assert!(hi[0] - lo[0] > 0.05);
    }

    fn teapot_calib_triangle_bytes(
        files: &std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Option<&[u8]> {
        if let Some(v) = files.get("base1/triangle.data") {
            return Some(v);
        }
        if let Some(v) = files.get("base1/filein2.data") {
            return Some(v);
        }
        files
            .iter()
            .find(|(k, _)| k.ends_with(".data") && k.contains("triangle"))
            .map(|(_, v)| v.as_slice())
    }

    #[test]
    fn loads_teapot_triangle_from_tox_when_present() {
        let tox = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = otd_td::decode_with_fallback(&tox).expect("decode");
        let bytes = teapot_calib_triangle_bytes(&decode.files).expect("triangle / filein2 sidecar");
        let fixture =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/triangle.data");
        if let Some(parent) = fixture.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&fixture, bytes);
        eprintln!("{}", dump_text(bytes));
        let g = parse(bytes).expect("parse triangle");
        assert_eq!(g.num_points(), 3, "calibration triangle");
        assert_eq!(g.indices.len() / 3, 1, "one triangle");
        let (lo, hi) = g.bounds();
        assert!(
            hi[0] - lo[0] > 0.5 && hi[1] - lo[1] > 0.5,
            "embedded triangle.data: expected ~2×3 from locked tox (compare examples/calib_tri.obj \
             only for ground truth). bounds {lo:?}..{hi:?}"
        );
        assert!(
            (hi[0] - lo[0] - 2.0).abs() < 0.05,
            "triangle width ~2: {:?}",
            (lo, hi)
        );
        assert!((hi[1] - lo[1] - 3.0).abs() < 0.05, "triangle height ~3");
    }

    #[test]
    fn loads_teapot_grid_from_tox_when_present() {
        let tox = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = otd_td::decode_with_fallback(&tox).expect("decode");
        let bytes = decode.files.get("base1/grid1.data").expect("grid1 sidecar");
        let g = parse(bytes).expect("parse grid1");
        assert_eq!(g.num_points(), 4, "2×2 grid");
        assert_eq!(g.indices.len() / 3, 2, "two triangles");
        let (lo, hi) = g.bounds();
        assert!(
            (hi[0] - lo[0] - 1.0).abs() < 0.05,
            "grid width {lo:?}..{hi:?}"
        );
        assert!(
            (hi[1] - lo[1] - 1.0).abs() < 0.05,
            "grid height {lo:?}..{hi:?}"
        );
        assert!(
            hi[2].abs() < 0.05 && lo[2].abs() < 0.05,
            "grid in Z=0 plane"
        );
    }

    #[test]
    fn loads_teapot_box_data_from_tox_when_present() {
        let tox = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../teapot.tox");
        if !tox.is_file() {
            return;
        }
        let decode = otd_td::decode_with_fallback(&tox).expect("decode");
        let bytes = decode.files.get("base1/box.data").expect("box sidecar");
        let fixture =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/box.data");
        if let Some(parent) = fixture.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&fixture, bytes);
        let g = parse(bytes).expect("parse box.data");
        eprintln!(
            "box.data: {} pts, {} tris, bounds {:?}..{:?}",
            g.num_points(),
            g.indices.len() / 3,
            g.bounds().0,
            g.bounds().1
        );
        let (lo, hi) = g.bounds();
        let tris = g.indices.len() / 3;
        if tris >= 100 {
            assert_eq!(g.num_points(), 96, "one copy of each sample per face");
            assert_eq!(tris, 108, "subdivided box surface, got {tris} tris");
            for p in &g.points {
                let axis = p.normal.iter().filter(|c| c.abs() > 0.99).count();
                let flat = p.normal.iter().filter(|c| c.abs() < 1e-4).count();
                assert_eq!((axis, flat), (1, 2), "face normal {:?}", p.normal);
            }
            for tri in g.indices.chunks(3) {
                let a = g.points[tri[0] as usize].position;
                let b = g.points[tri[1] as usize].position;
                let c = g.points[tri[2] as usize].position;
                let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                let geo = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let n = g.points[tri[0] as usize].normal;
                let dot = geo[0] * n[0] + geo[1] * n[1] + geo[2] * n[2];
                assert!(dot > 0.0, "winding opposes normal {n:?}");
            }
        } else {
            assert_eq!(g.num_points(), 24);
            assert_eq!(tris, 12, "unit box fallback");
        }
        assert!(
            (hi[0] - lo[0] - 1.0).abs() < 0.08,
            "box width ~1: {lo:?}..{hi:?}"
        );
        let extent_y = hi[1] - lo[1];
        let extent_z = hi[2] - lo[2];
        let max_extent = 1.05;
        assert!(extent_y <= max_extent, "box height {extent_y}");
        assert!(extent_z <= max_extent, "box depth {extent_z}");
    }

    #[test]
    fn loads_protomarquee_structure_fixture_when_present() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/structure.data");
        if !path.exists() {
            return;
        }
        let g = load(&path).expect("parse structure");
        assert!(g.num_points() > 50);
        assert!(g.indices.len() / 3 > 50);
        let (lo, hi) = g.bounds();
        assert!(hi[1] - lo[1] > 1.0);
    }
