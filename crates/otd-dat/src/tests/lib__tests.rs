use super::*;

use super::*;

    #[test]
    fn a_table_round_trips_through_delimited_text() {
        let d = DatData::from_delimited("name\tvalue\na\t1\nb\t2", '\t');
        assert_eq!(d.num_rows(), 3);
        assert_eq!(d.num_cols(), 2);
        assert_eq!(d.cell(1, 0), "a");
        assert_eq!(d.lookup("b", 1), Some("2"));
        assert_eq!(d.as_text(), "name\tvalue\na\t1\nb\t2");
    }

    #[test]
    fn ragged_rows_read_as_empty_rather_than_panicking() {
        let d = DatData::table(vec![vec!["a".into()], vec!["b".into(), "c".into()]]);
        assert_eq!(d.num_cols(), 2);
        assert_eq!(d.cell(0, 1), "");
        assert_eq!(d.cell(9, 9), "");
    }

    #[test]
    fn a_text_dat_is_one_cell() {
        let d = DatData::text("hello\nworld");
        assert!(d.is_text);
        assert_eq!(d.num_rows(), 1);
        assert_eq!(d.as_text(), "hello\nworld");
    }
