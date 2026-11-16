use super::{OscArg, encode_message};

    #[test]
    fn encodes_address_and_typetags() {
        let buf = encode_message("/test", &[OscArg::Int(1)]).unwrap();
        assert!(buf.starts_with(b"/test\0"));
        assert!(buf.windows(2).any(|w| w == b",i"));
    }
