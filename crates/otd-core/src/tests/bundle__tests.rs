use super::*;
    use crate::component::Component;
    use crate::graph::{Connector, Family, OpDef};
    use crate::param::Param;

    pub(super) fn registry() -> OpRegistry {
        fn none() -> indexmap::IndexMap<String, Param> {
            indexmap::IndexMap::new()
        }
        let mut r = OpRegistry::new();
        r.register(OpDef {
            type_name: "container",
            input_families: &[],
            label: "Container",
            family: Family::Comp,
            inputs: &[],
            summary: "",
            time_dependent: false,
            params: none,
            connector: Connector::None,
        });
        fn with_file() -> indexmap::IndexMap<String, Param> {
            let mut m = indexmap::IndexMap::new();
            m.insert("file".into(), Param::str("").as_file_ref());
            m
        }
        r.register(OpDef {
            type_name: "player",
            input_families: &[],
            label: "Player",
            family: Family::Top,
            inputs: &[],
            summary: "",
            time_dependent: true,
            params: with_file,
            connector: Connector::None,
        });
        r.register(OpDef {
            type_name: "pass",
            input_families: &[],
            label: "Pass",
            family: Family::Top,
            inputs: &["in"],
            summary: "",
            time_dependent: false,
            params: none,
            connector: Connector::None,
        });
        r
    }

    pub(super) fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("otd-bundle-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A `.otdc` holding one operator, saved somewhere the bundle is not.
    fn write_component(dir: &Path, name: &str) -> PathBuf {
        let reg = registry();
        let mut g = Graph::new();
        let comp = g
            .create(g.root(), reg.get("container").unwrap(), None)
            .unwrap();
        g.create(comp, reg.get("pass").unwrap(), None).unwrap();
        let path = dir.join(format!("{name}.otdc"));
        Component::from_graph(&g, comp, &reg)
            .unwrap()
            .save(&path)
            .unwrap();
        path
    }

    #[test]
    fn an_exported_bundle_opens_from_anywhere() {
        let reg = registry();
        let elsewhere = scratch("elsewhere");
        let source = write_component(&elsewhere, "meter");

        let mut g = Graph::new();
        let comp = g
            .create(g.root(), reg.get("container").unwrap(), None)
            .unwrap();
        g.attach_external(comp, &source.to_string_lossy(), &reg)
            .unwrap();

        let out = scratch("out");
        let bundle = export(&g, &reg, 60.0, 48_000.0, &out, "show").unwrap();
        assert!(bundle.missing.is_empty(), "{:?}", bundle.missing);
        assert_eq!(bundle.components.len(), 1);
        assert!(out.join("components/meter.otdc").exists());

        // The point of the exercise: the component the bundle now depends on
        // is gone from where it was authored, and the bundle still opens.
        std::fs::remove_dir_all(&elsewhere).unwrap();
        let loaded = Project::open(&bundle.project, &reg).unwrap();
        let id = loaded.find("/container1").unwrap();
        assert_eq!(
            loaded.node(id).external.as_deref(),
            Some("components/meter.otdc"),
            "the reference is relative, so the folder can move"
        );
        assert_eq!(
            loaded.node(id).children.len(),
            1,
            "the component's network came back with it"
        );
    }

    #[test]
    fn exporting_does_not_disturb_the_project_being_worked_in() {
        let reg = registry();
        let elsewhere = scratch("keep-elsewhere");
        let source = write_component(&elsewhere, "meter");

        let mut g = Graph::new();
        let comp = g
            .create(g.root(), reg.get("container").unwrap(), None)
            .unwrap();
        g.attach_external(comp, &source.to_string_lossy(), &reg)
            .unwrap();

        export(&g, &reg, 60.0, 48_000.0, scratch("keep-out"), "show").unwrap();
        assert_eq!(
            g.node(comp).external.as_deref(),
            Some(source.to_string_lossy().as_ref()),
            "the open project still points at the shared file it is editing"
        );
    }

    #[test]
    fn one_component_used_twice_is_copied_once() {
        let reg = registry();
        let elsewhere = scratch("twice-src");
        let source = write_component(&elsewhere, "meter");

        let mut g = Graph::new();
        let mut ids = Vec::new();
        for _ in 0..2 {
            let c = g
                .create(g.root(), reg.get("container").unwrap(), None)
                .unwrap();
            g.attach_external(c, &source.to_string_lossy(), &reg)
                .unwrap();
            ids.push(c);
        }

        let out = scratch("twice-out");
        let bundle = export(&g, &reg, 60.0, 48_000.0, &out, "show").unwrap();
        assert_eq!(bundle.components.len(), 1, "one file, two instances");

        let loaded = Project::open(&bundle.project, &reg).unwrap();
        for path in ["/container1", "/container2"] {
            let id = loaded.find(path).unwrap();
            assert_eq!(
                loaded.node(id).external.as_deref(),
                Some("components/meter.otdc"),
                "{path} shares the one copy"
            );
        }
    }

    #[test]
    fn two_different_components_with_the_same_name_do_not_collide() {
        let reg = registry();
        let a_dir = scratch("collide-a");
        let b_dir = scratch("collide-b");
        let a = write_component(&a_dir, "meter");
        let b = write_component(&b_dir, "meter");

        let mut g = Graph::new();
        for source in [&a, &b] {
            let c = g
                .create(g.root(), reg.get("container").unwrap(), None)
                .unwrap();
            g.attach_external(c, &source.to_string_lossy(), &reg)
                .unwrap();
        }

        let out = scratch("collide-out");
        let bundle = export(&g, &reg, 60.0, 48_000.0, &out, "show").unwrap();
        // Two distinct networks that happen to share a file name must stay
        // two files — overwriting one with the other would point an instance
        // at somebody else's component and look like it worked.
        assert_eq!(bundle.components.len(), 2, "{:?}", bundle.components);
        assert!(out.join("components/meter.otdc").exists());
        assert!(out.join("components/meter1.otdc").exists());
    }

    #[test]
    fn a_missing_component_is_listed_rather_than_aborting_the_export() {
        let reg = registry();
        let mut g = Graph::new();
        let comp = g
            .create(g.root(), reg.get("container").unwrap(), None)
            .unwrap();
        // Reference a file that is not there, the way a half-reorganised
        // project does.
        g.node_mut_quiet(comp).external = Some("/nowhere/gone.otdc".into());

        let out = scratch("missing-out");
        let bundle = export(&g, &reg, 60.0, 48_000.0, &out, "show").unwrap();
        assert_eq!(bundle.missing.len(), 1);
        assert_eq!(bundle.missing[0].1, "/nowhere/gone.otdc");
        assert!(
            bundle.project.exists(),
            "the rest of the bundle is still written, so the list is actionable"
        );
    }
