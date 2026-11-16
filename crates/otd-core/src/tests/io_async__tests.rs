use super::*;

    #[test]
    fn trim_front_keeps_tail() {
        let mut v = vec![0, 1, 2, 3];
        trim_front(&mut v, 2);
        assert_eq!(v, [2, 3]);
    }

    #[test]
    fn edge_inbox_respects_cap() {
        let edge = EdgeInbox::new(2);
        edge.push(1);
        edge.push(2);
        edge.push(3);
        let mut hold = Vec::new();
        let batch = edge.drain_for_cook(&mut hold);
        assert_eq!(batch, [2, 3]);
        assert_eq!(hold, [2, 3]);
    }
