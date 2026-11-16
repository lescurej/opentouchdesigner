use super::*;

    #[test]
    fn ffmpeg_is_found_with_the_path_a_double_clicked_app_gets() {
        // The bug this guards: launched from Finder, the app inherits only
        // the system directories, so a Homebrew ffmpeg vanishes and every
        // media node claims the tool is not installed on a machine where it
        // is plainly there.
        //
        // Skipped where ffmpeg genuinely is not installed — there is nothing
        // to find, and this is a search test, not an ffmpeg test.
        if ffmpeg().is_none() {
            return;
        }
        let bundle = std::ffi::OsStr::new("/usr/bin:/bin:/usr/sbin:/sbin");
        let found = find_in("ffmpeg", bundle, MEDIA_DIRECTORIES);
        assert!(
            found.is_some(),
            "ffmpeg is installed but was not found with a Finder-style PATH"
        );
        assert!(is_executable(&found.unwrap()));
    }

    #[test]
    fn the_search_path_wins_over_the_fallbacks() {
        // A build somebody put on their PATH on purpose must not lose to
        // whatever an installer left in /usr/local/bin.
        let dir = std::env::temp_dir().join("otd-tools-test");
        let _ = std::fs::create_dir_all(&dir);
        let chosen = dir.join("otd-fake-tool");
        std::fs::write(&chosen, b"#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&chosen, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let found = find_in(
            "otd-fake-tool",
            dir.as_os_str(),
            &["/usr/bin", "/usr/local/bin"],
        );
        assert_eq!(found.as_deref(), Some(chosen.as_path()));
        let _ = std::fs::remove_file(&chosen);
    }

    #[test]
    fn a_tool_that_is_not_anywhere_is_none() {
        assert!(find("otd-definitely-not-a-real-tool", MEDIA_DIRECTORIES).is_none());
    }
