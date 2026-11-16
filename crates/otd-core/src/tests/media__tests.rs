use super::*;
    use crate::Graph;

    #[test]
    fn default_movie_file_resolves_from_shipped_media_dir() {
        let graph = Graph::new();
        let path = resolve_media_path(&graph, "");
        assert!(
            path.is_file(),
            "expected {} on disk, got {}",
            MOVIE_IN_DEFAULT_FILE,
            path.display()
        );
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some(MOVIE_IN_DEFAULT_FILE)
        );
    }

    #[test]
    fn project_relative_path_wins_over_bundled_default() {
        let dir = std::env::temp_dir().join(format!("otd-media-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let clip = dir.join("mine.png");
        std::fs::write(&clip, b"fake").unwrap();
        let mut graph = Graph::new();
        graph.set_base_dir(Some(dir.clone()));
        let path = resolve_media_path(&graph, "mine.png");
        assert_eq!(path, clip);
        let _ = std::fs::remove_dir_all(&dir);
    }
