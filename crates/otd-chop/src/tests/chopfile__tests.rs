use super::*;

    #[test]
    fn round_trip_och_format() {
        let data = ChopData::new(
            vec![
                Channel::new("a", vec![0.0, 0.5]),
                Channel::new("b", vec![1.0]),
            ],
            48000.0,
            false,
        );
        let text = serialize(&data);
        let back = parse(&text).unwrap();
        assert_eq!(back.sample_rate, 48000.0);
        assert_eq!(back.channels[0].as_slice(), &[0.0, 0.5]);
    }
