use super::*;
    use std::fs;

    #[test]
    fn td_columns_and_depth() {
        let root = std::env::temp_dir().join(format!("otd-folder-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("a.txt"), b"hi").unwrap();
        fs::write(root.join("sub/b.txt"), b"bye").unwrap();
        let cols = column_flags(
            true, false, false, true, true, true, false, true, true, false, false, false,
        );
        let cfg = scan_config(&root, "*", true, 0, false, 1, 2, false);
        let table = cook(&root, &cfg, cols, true).unwrap();
        assert_eq!(table.cell(0, 0), "name");
        assert_eq!(table.cell(0, 3), "depth");
        assert_eq!(table.cell(0, 4), "path");
        assert!(table.num_rows() >= 3);
        let _ = fs::remove_dir_all(&root);
    }
