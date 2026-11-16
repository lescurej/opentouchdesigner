use super::*;
    use std::fs;

    #[test]
    fn recognizes_a_python_build_standalone_layout() {
        let root = std::env::temp_dir().join("otd-python-home-test");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("lib/python3.12")).unwrap();
        fs::write(root.join("bin/python3"), b"").unwrap();
        assert!(is_python_home(&root));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_an_empty_directory() {
        let root = std::env::temp_dir().join("otd-python-home-empty");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        assert!(!is_python_home(&root));
        let _ = fs::remove_dir_all(&root);
    }
