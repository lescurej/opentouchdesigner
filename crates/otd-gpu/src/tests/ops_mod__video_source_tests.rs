use super::nothing_to_open;

    #[test]
    fn a_blank_device_means_the_default_camera_not_nothing() {
        assert!(!nothing_to_open(true, ""), "blank device = default camera");
        assert!(!nothing_to_open(true, "   "));
        assert!(!nothing_to_open(true, "1"), "a named device, obviously");

        assert!(nothing_to_open(false, ""));
        assert!(nothing_to_open(false, "  \t "));
        assert!(!nothing_to_open(false, "/clips/a.mov"));
    }
