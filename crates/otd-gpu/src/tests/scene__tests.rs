use super::*;

use super::*;
    use otd_core::Graph;

    struct FakeScene(Vec<(String, Vec<f32>)>);
    impl Scene for FakeScene {
        fn geometry(&self, _path: &str) -> Option<&Geometry> {
            None
        }
        fn channels(&self, path: &str) -> Option<Vec<(String, Vec<f32>)>> {
            (path == "/pos").then(|| self.0.clone())
        }
    }

    fn geometry_node(graph: &mut Graph) -> otd_core::NodeId {
        let mut reg = OpRegistry::new();
        register_all(&mut reg);
        let root = graph.root();
        graph
            .create(root, reg.get(GEOMETRY).unwrap(), Some("geo1"))
            .unwrap()
    }

    #[test]
    fn instancing_off_gives_exactly_one_instance() {
        let mut graph = Graph::new();
        let id = geometry_node(&mut graph);
        let scene = FakeScene(vec![]);
        let list = instances(graph.node(id), &EvalContext::default(), &scene);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].scale, [1.0; 3]);
    }

    #[test]
    fn each_sample_of_a_chop_is_an_instance() {
        let mut graph = Graph::new();
        let id = geometry_node(&mut graph);
        graph
            .set_param(id, "instancing", Value::Bool(true))
            .unwrap();
        graph
            .set_param(id, "instancechop", Value::Str("/pos".into()))
            .unwrap();

        let scene = FakeScene(vec![
            ("tx".into(), vec![0.0, 1.0, 2.0, 3.0]),
            ("ty".into(), vec![10.0, 11.0, 12.0, 13.0]),
        ]);
        let list = instances(graph.node(id), &EvalContext::default(), &scene);
        assert_eq!(list.len(), 4);
        assert_eq!(list[2].translate, [2.0, 12.0, 0.0]);
        // Unreferenced channels keep their defaults.
        assert_eq!(list[2].scale, [1.0; 3]);
    }

    #[test]
    fn a_single_sample_channel_broadcasts_across_instances() {
        let mut graph = Graph::new();
        let id = geometry_node(&mut graph);
        graph
            .set_param(id, "instancing", Value::Bool(true))
            .unwrap();
        graph
            .set_param(id, "instancechop", Value::Str("/pos".into()))
            .unwrap();
        graph
            .set_param(id, "sx", Value::Str("size".into()))
            .unwrap();

        let scene = FakeScene(vec![
            ("tx".into(), vec![0.0, 1.0, 2.0]),
            // One sample: the "all of them scale together" case.
            ("size".into(), vec![0.25]),
        ]);
        let list = instances(graph.node(id), &EvalContext::default(), &scene);
        assert_eq!(list.len(), 3);
        assert!(list.iter().all(|i| i.scale[0] == 0.25));
    }

    #[test]
    fn a_missing_instance_chop_falls_back_to_one_instance() {
        let mut graph = Graph::new();
        let id = geometry_node(&mut graph);
        graph
            .set_param(id, "instancing", Value::Bool(true))
            .unwrap();
        graph
            .set_param(id, "instancechop", Value::Str("/nowhere".into()))
            .unwrap();
        let scene = FakeScene(vec![]);
        assert_eq!(
            instances(graph.node(id), &EvalContext::default(), &scene).len(),
            1
        );
    }

    #[test]
    fn a_geometry_comps_references_are_cook_dependencies() {
        let mut graph = Graph::new();
        let id = geometry_node(&mut graph);
        graph
            .set_param(id, "sop", Value::Str("/sphere1".into()))
            .unwrap();
        graph
            .set_param(id, "instancechop", Value::Str("/pos".into()))
            .unwrap();
        let sources: Vec<&str> = graph.node(id).param_sources().collect();
        assert!(sources.contains(&"/sphere1"), "{sources:?}");
        assert!(sources.contains(&"/pos"), "{sources:?}");
    }
