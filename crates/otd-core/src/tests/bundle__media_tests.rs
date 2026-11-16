use super::*;

use super::tests::*;
    use crate::value::Value;

    /// A movie is as much a dependency as a component, and by the same
    /// argument: the authoring path is somebody's scratch folder.
    #[test]
    fn a_bundle_brings_the_media_with_it_and_rewrites_the_paths() {
        let reg = registry();
        let elsewhere = scratch("media-source");
        let clip = elsewhere.join("plate.mov");
        std::fs::write(&clip, b"not really a movie, but a real file").unwrap();

        let mut g = Graph::new();
        let a = g
            .create(g.root(), reg.get("player").unwrap(), Some("a"))
            .unwrap();
        let b = g
            .create(g.root(), reg.get("player").unwrap(), Some("b"))
            .unwrap();
        g.set_param(a, "file", Value::Str(clip.to_string_lossy().into()))
            .unwrap();
        // The same file on two nodes is one copy, and both point at it.
        g.set_param(b, "file", Value::Str(clip.to_string_lossy().into()))
            .unwrap();

        let out = scratch("media-out");
        let bundle = export(&g, &reg, 60.0, 48_000.0, &out, "show").unwrap();
        assert!(bundle.missing.is_empty(), "{:?}", bundle.missing);
        assert_eq!(bundle.media.len(), 1, "one file, copied once");
        assert!(out.join("media/plate.mov").exists());

        // The source folder is gone; the bundle still knows where its movie is.
        std::fs::remove_dir_all(&elsewhere).unwrap();
        let loaded = Project::open(&bundle.project, &reg).unwrap();
        for name in ["/a", "/b"] {
            let id = loaded.find(name).unwrap();
            assert_eq!(
                loaded.node(id).param("file").unwrap().value.as_str(),
                "media/plate.mov",
                "{name} should point inside the bundle"
            );
        }
        assert_eq!(
            loaded.resolve_external("media/plate.mov"),
            out.join("media/plate.mov"),
            "and that relative path resolves against the bundle"
        );
    }

    #[test]
    fn exporting_does_not_rewrite_the_project_being_worked_in() {
        let reg = registry();
        let elsewhere = scratch("media-keep");
        let clip = elsewhere.join("plate.mov");
        std::fs::write(&clip, b"x").unwrap();

        let mut g = Graph::new();
        let a = g
            .create(g.root(), reg.get("player").unwrap(), Some("a"))
            .unwrap();
        g.set_param(a, "file", Value::Str(clip.to_string_lossy().into()))
            .unwrap();

        export(&g, &reg, 60.0, 48_000.0, scratch("media-keep-out"), "show").unwrap();
        assert_eq!(
            g.node(a).param("file").unwrap().value.as_str(),
            clip.to_string_lossy(),
            "the open project still points at the file the artist is using"
        );
    }

    #[test]
    fn a_missing_movie_is_listed_rather_than_thrown() {
        let reg = registry();
        let mut g = Graph::new();
        let a = g
            .create(g.root(), reg.get("player").unwrap(), Some("a"))
            .unwrap();
        g.set_param(a, "file", Value::Str("/no/such/clip.mov".into()))
            .unwrap();

        // An artist mid-reorganisation should get a list of what to fix, not
        // one error and no bundle.
        let bundle = export(&g, &reg, 60.0, 48_000.0, scratch("media-missing"), "show").unwrap();
        assert!(bundle.project.exists());
        assert_eq!(bundle.missing.len(), 1);
        assert!(bundle.missing[0].1.contains("clip.mov"));
    }

    #[test]
    fn an_empty_file_parameter_is_not_a_dependency() {
        let reg = registry();
        let mut g = Graph::new();
        g.create(g.root(), reg.get("player").unwrap(), Some("a"))
            .unwrap();
        let bundle = export(&g, &reg, 60.0, 48_000.0, scratch("media-empty"), "show").unwrap();
        assert!(bundle.media.is_empty());
        assert!(bundle.missing.is_empty());
    }
