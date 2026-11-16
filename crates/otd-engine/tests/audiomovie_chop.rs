//! Audio Movie CHOP follows a Movie File In TOP.

use otd_chop::io::AUDIO_MOVIE;
use otd_core::{CookContext, CookEngine, Graph, Value};
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

fn add(graph: &mut Graph, reg: &otd_core::OpRegistry, op: &str, name: &str) -> otd_core::NodeId {
    let root = graph.root();
    let def = reg.get(op).unwrap().clone();
    graph.create(root, &def, Some(name)).unwrap()
}

#[test]
fn audiomovie_outputs_audio_from_a_referenced_movie_top() {
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
    let dir = std::env::temp_dir().join(format!("otd-audiomovie-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tone.mp4");
    std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=64x64:d=1",
            "-shortest",
            "-pix_fmt",
            "yuv420p",
            path.to_str().unwrap(),
        ])
        .output()
        .expect("ffmpeg");

    let mut graph = Graph::new();
    graph.set_base_dir(Some(dir.clone()));
    let movie = add(&mut graph, &reg, "moviefileinTOP", "movie1");
    graph
        .set_param(movie, "file", Value::Str("tone.mp4".into()))
        .unwrap();
    let audio = add(&mut graph, &reg, AUDIO_MOVIE, "audio1");
    graph
        .set_param(audio, "top", Value::Str("../movie1".into()))
        .unwrap();

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut heard = false;
    // The soundtrack is read on the movie's AVFoundation thread. Cook until
    // those samples show up, or give up after a few seconds.
    let mut time = CookContext::default();
    for _ in 0..150 {
        engines.begin_frame();
        engines
            .run_cook_frame(&mut cook, &graph, &[audio, movie], &time)
            .unwrap();
        engines.end_frame(&graph);
        if let Some(cooked) = engines.chop_data(audio) {
            let peak = cooked
                .channels
                .iter()
                .flat_map(|c| c.samples.iter())
                .copied()
                .fold(0.0f32, |a, b| a.max(b.abs()));
            if peak > 0.01 {
                heard = true;
                break;
            }
        }
        time.advance(time.dt);
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(
        heard,
        "soundtrack never arrived from the movie reader (audio: {:?}, movie: {:?})",
        engines.node_status(&graph, audio),
        engines.node_status(&graph, movie),
    );

    let data = engines.chop_data(audio).expect("audiomovie should cook");
    assert!(
        !data.channels.is_empty(),
        "expected at least one audio channel"
    );
    assert!(
        data.num_samples() > 1,
        "expected a time slice of audio samples"
    );
    let peak = data
        .channels
        .iter()
        .flat_map(|c| c.samples.iter())
        .copied()
        .fold(0.0f32, |a, b| a.max(b.abs()));
    assert!(peak > 0.01, "expected non-silent audio, peak was {peak}");

    std::fs::remove_dir_all(&dir).ok();
}
