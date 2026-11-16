//! Ramp TOP reads gradient stops from a referenced Table DAT.

use otd_core::{CookContext, CookEngine, Graph, Value};
use otd_engine::{Engines, registry};
use otd_gpu::{GpuContext, read_pixels_rgba8};

fn px(pixels: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

#[test]
fn ramp_samples_a_three_stop_dat_at_the_middle() {
    let gpu = match GpuContext::headless() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("skipping: no GPU ({e})");
            return;
        }
    };
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let table = graph
        .create(root, reg.get("tableDAT").unwrap(), Some("keys"))
        .unwrap();
    graph
        .set_param(
            table,
            "text",
            Value::Str("pos\tr\tg\tb\ta\n0\t0\t0\t0\t1\n0.5\t1\t0\t0\t1\n1\t1\t1\t1\t1".into()),
        )
        .unwrap();
    let ramp = graph
        .create(root, reg.get("rampTOP").unwrap(), Some("ramp1"))
        .unwrap();
    graph
        .set_param(ramp, "dat", Value::Str("/keys".into()))
        .unwrap();
    graph.set_param(ramp, "resw", Value::Int(64)).unwrap();
    graph.set_param(ramp, "resh", Value::Int(8)).unwrap();

    let mut engines = Engines::new(gpu.clone());
    let mut cook = CookEngine::new();
    let ctx = CookContext::default();
    engines.begin_frame();
    cook.cook_frame(&graph, &[table, ramp], &ctx, &mut engines)
        .unwrap();
    engines.end_frame(&graph);

    let tex = engines.top.output(&graph, ramp).unwrap().clone();
    let (w, _, pixels) = read_pixels_rgba8(&gpu, &tex).unwrap();
    let mid = px(&pixels, w, 32, 4);
    assert!(
        mid[0] > 200 && mid[1] < 40,
        "middle stop should be red, got {:?}",
        mid
    );
}
