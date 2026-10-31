use otd_core::{CookContext, CookEngine, Graph, NodeId, Value};
use otd_engine::{Engines, registry};

macro_rules! gpu_or_skip {
    () => {
        match otd_gpu::GpuContext::headless() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skipping: no GPU available ({e})");
                return;
            }
        }
    };
}

fn add(graph: &mut Graph, reg: &otd_core::OpRegistry, op: &str, name: &str) -> NodeId {
    let root = graph.root();
    let def = reg.get(op).unwrap().clone();
    graph.create(root, &def, Some(name)).unwrap()
}

#[test]
fn info_chop_reads_movie_file_in_channels() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let dir = std::env::temp_dir().join(format!("otd-info-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tile.png");
    let mut buffer = image::RgbaImage::new(64, 32);
    for px in buffer.pixels_mut() {
        *px = image::Rgba([128, 64, 32, 255]);
    }
    buffer.save(&path).unwrap();

    let mut graph = Graph::new();
    let movie = add(&mut graph, &reg, "moviefileinTOP", "movie1");
    graph
        .set_param(movie, "file", Value::Str(path.display().to_string()))
        .unwrap();
    let info = add(&mut graph, &reg, "infoCHOP", "info1");
    graph
        .set_param(info, "op", Value::Str("../movie1".into()))
        .unwrap();

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    for _ in 0..8 {
        let time = CookContext::default();
        engines.begin_frame();
        engines
            .run_cook_frame(&mut cook, &graph, &[info, movie], &time)
            .unwrap();
        engines.end_frame();
    }

    let data = engines.chop_data(info).expect("info chop cooked");
    assert_eq!(data.value("resx"), Some(64.0));
    assert_eq!(data.value("resy"), Some(32.0));
    assert_eq!(data.value("open"), Some(1.0));
    assert_eq!(data.value("index"), Some(0.0));
    assert!(data.value("total_cooks").unwrap_or(0.0) >= 1.0);
    assert!(data.value("num_channels").is_none());
    assert_eq!(data.value("resx"), Some(64.0));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn info_tracks_movie_with_full_graph_roots_and_no_viewer_flag() {
    let gpu = gpu_or_skip!();
    if !std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        eprintln!("skipping: ffmpeg not installed");
        return;
    }
    let reg = registry();
    let dir = std::env::temp_dir().join(format!("otd-info-mv-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clip.mp4");
    std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=64x64:rate=25:duration=1",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("ffmpeg");

    let mut graph = Graph::new();
    let movie = add(&mut graph, &reg, "moviefileinTOP", "movie1");
    graph.node_mut(movie).flags.viewer = false;
    graph
        .set_param(movie, "file", Value::Str(path.display().to_string()))
        .unwrap();
    graph.set_param(movie, "play", Value::Bool(true)).unwrap();
    let info = add(&mut graph, &reg, "infoCHOP", "info1");
    graph
        .set_param(info, "op", Value::Str("../movie1".into()))
        .unwrap();

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut time = CookContext::default();
    let roots = otd_engine::editor_cook_roots(&graph, None);
    let mut first = None;
    let mut last = None;
    for _ in 0..30 {
        engines.begin_frame();
        engines
            .run_cook_frame(&mut cook, &graph, &roots, &time)
            .unwrap();
        engines.end_frame();
        if let Some(v) = engines.chop_data(info).and_then(|d| d.value("index")) {
            if first.is_none() {
                first = Some(v);
            }
            last = Some(v);
        }
        time.advance(1.0 / 60.0);
    }
    let first = first.expect("index channel");
    let last = last.expect("index advanced");
    assert!(
        last > first + 0.5,
        "info index should track movie without viewer/on-screen roots (first={first}, last={last})"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn info_chop_emits_one_sample_per_channel() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let dir = std::env::temp_dir().join(format!("otd-info-ns-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tile.png");
    image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 255]))
        .save(&path)
        .unwrap();

    let mut graph = Graph::new();
    let movie = add(&mut graph, &reg, "moviefileinTOP", "movie1");
    graph
        .set_param(movie, "file", Value::Str(path.display().to_string()))
        .unwrap();
    let info = add(&mut graph, &reg, "infoCHOP", "info1");
    graph
        .set_param(info, "op", Value::Str("../movie1".into()))
        .unwrap();

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut time = CookContext::default();
    for i in 0..20 {
        time.dt = if i % 3 == 0 { 2.0 / 60.0 } else { 1.0 / 60.0 };
        engines.begin_frame();
        engines
            .run_cook_frame(
                &mut cook,
                &graph,
                &otd_engine::editor_cook_roots(&graph, None),
                &time,
            )
            .unwrap();
        engines.end_frame();
        let data = engines.chop_data(info).expect("info cooked");
        assert!(
            !data.time_sliced,
            "info should not time-slice movie scalars"
        );
        assert_eq!(
            data.num_samples(),
            1,
            "variable frame dt should not change info sample count (got {})",
            data.num_samples()
        );
        time.advance(time.dt);
    }

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn info_index_slope_is_media_fps_at_project_rate() {
    let gpu = gpu_or_skip!();
    if !std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        eprintln!("skipping: ffmpeg not installed");
        return;
    }
    let reg = registry();
    let dir = std::env::temp_dir().join(format!("otd-slope-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clip.mp4");
    std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc=size=64x64:rate=25:duration=2",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("ffmpeg");

    let mut graph = Graph::new();
    let movie = add(&mut graph, &reg, "moviefileinTOP", "movie1");
    graph
        .set_param(movie, "file", Value::Str(path.display().to_string()))
        .unwrap();
    graph.set_param(movie, "play", Value::Bool(true)).unwrap();
    let info = add(&mut graph, &reg, "infoCHOP", "info1");
    graph
        .set_param(info, "op", Value::Str("../movie1".into()))
        .unwrap();
    let slope = add(&mut graph, &reg, "slopeCHOP", "slope1");
    graph.connect(info, slope, 0).unwrap();

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut time = CookContext::default();
    let roots = otd_engine::editor_cook_roots(&graph, None);
    for i in 0..40 {
        time.dt = if i % 5 == 0 { 0.12 } else { 1.0 / 60.0 };
        engines.begin_frame();
        engines
            .run_cook_frame(&mut cook, &graph, &roots, &time)
            .unwrap();
        engines.end_frame();
        time.advance(time.dt);
    }
    let v = engines
        .chop_data(slope)
        .and_then(|d| d.value("index"))
        .unwrap_or(f32::NAN);
    assert!(
        (v - 25.0).abs() < 2.0,
        "slope on index should be ~25 at 25fps media, got {v}"
    );

    std::fs::remove_dir_all(&dir).ok();
}
