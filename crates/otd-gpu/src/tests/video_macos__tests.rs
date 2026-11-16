
use super::{decode_file_loop, probe, spawn_file_reader};
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::sync_channel;
    use std::time::Duration;

    fn plasma_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/media/plasma.mp4")
    }

    #[test]
    fn avfoundation_probes_plasma() {
        let path = plasma_path();
        if !path.is_file() {
            return;
        }
        let info = probe(path.to_str().unwrap()).expect("probe");
        assert!(info.width > 0 && info.height > 0);
        assert!(info.fps > 0.0);
    }

    #[test]
    fn avfoundation_decodes_one_frame() {
        let path = plasma_path();
        if !path.is_file() {
            return;
        }
        let path = path.to_str().unwrap().to_string();
        let info = probe(&path).expect("probe");
        let stop = Arc::new(AtomicBool::new(false));
        let (tx, rx) = sync_channel(4);
        let worker_stop = stop.clone();
        let worker_path = path.clone();
        let worker_fps = info.fps;
        let reader_slot = Arc::new(std::sync::Mutex::new(None));
        let decode_io = Arc::new(std::sync::Mutex::new(()));
        std::thread::spawn(move || {
            let _ = decode_file_loop(
                &worker_path,
                worker_fps,
                0,
                &tx,
                &worker_stop,
                reader_slot,
                decode_io,
            );
        });
        let (_idx, frame) = rx.recv_timeout(Duration::from_secs(30)).expect("frame");
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(frame.width, info.width);
        assert_eq!(frame.height, info.height);
        assert_eq!(frame.pixels.len(), (info.width * info.height * 4) as usize);
    }

    #[test]
    fn avfoundation_reader_thread_starts() {
        let path = plasma_path();
        if !path.is_file() {
            return;
        }
        let path = path.to_str().unwrap();
        let info = probe(path).expect("probe");
        let problem = Arc::new(std::sync::Mutex::new(None));
        let reader = spawn_file_reader(path, &info, 0, 8, problem.clone()).expect("spawn");
        drop(reader);
        assert!(problem.lock().unwrap().is_none());
    }
