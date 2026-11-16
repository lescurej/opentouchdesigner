use super::*;

    #[test]
    fn movie_lag_telemetry_is_not_a_node_message() {
        assert!(is_movie_playback_telemetry(
            "lag 2 · pending 1 · decode on · drops 0"
        ));
    }

    #[test]
    fn movie_lag_is_a_warning_not_an_error() {
        assert_eq!(
            classify_node_message("lag 2 · pending 1 · decode on · drops 0"),
            NodeMessageLevel::Warning
        );
    }

    #[test]
    fn missing_file_is_an_error() {
        assert_eq!(
            classify_node_message("could not open movie"),
            NodeMessageLevel::Error
        );
    }
