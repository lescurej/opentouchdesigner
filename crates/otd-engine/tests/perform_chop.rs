use otd_chop::data::{slice_len, CONTROL_RATE};
use otd_chop::ops::PERFORM;
use otd_core::{CookContext, CookEngine, Graph, OpRegistry};
use otd_engine::{Engines, clip_frame_dt, registry, tick_frame_state, FrameConfig, PlaybackState};
use otd_gpu::GpuContext;

fn add(graph: &mut Graph, reg: &OpRegistry, op: &str, name: &str) -> otd_core::NodeId {
    let root = graph.root();
    let def = reg.get(op).unwrap().clone();
    graph.create(root, &def, Some(name)).unwrap()
}

#[test]
fn perform_chop_reports_time_slice_and_cookrate() {
    let gpu = match GpuContext::headless() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut graph = Graph::new();
    let reg = registry();
    let perform = add(&mut graph, &reg, PERFORM, "p1");

    let mut engines = Engines::new(gpu);
    let mut cook = CookEngine::new();
    let mut time = CookContext::default();
    let mut playback = PlaybackState {
        playing: true,
        ..Default::default()
    };

    let dt = clip_frame_dt(1.0 / 30.0, 200.0);
    tick_frame_state(
        &mut graph,
        &reg,
        &mut engines,
        &mut cook,
        &mut time,
        FrameConfig {
            dt,
            roots: vec![perform],
            audio_latency_ms: 0,
            input: Default::default(),
            display_blits: Vec::new(),
            defer_gpu_submit: false,
            max_time_slice_ms: 200.0,
            advance_timeline: true,
        },
        &mut playback,
    );

    let data = engines.channels.get(perform).expect("perform chop");
    let cookrate = data.channel("cookrate").unwrap().last();
    let step = data.channel("timeslicestep").unwrap().last();
    let msec = data.channel("msec").unwrap().last();

    assert!((cookrate - 60.0).abs() < 0.01);
    assert!((step - 2.0).abs() < 0.01, "30fps project 1/30s slice => 2 frames at 60");
    assert!(msec >= 0.0);
    assert_eq!(data.num_samples(), slice_len(CONTROL_RATE, &time));
}