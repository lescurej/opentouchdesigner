use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

use otd_core::{CookContext, CookEngine, Graph, NodeId, OpRegistry, Value};
use otd_engine::{Engines, registry};
use otd_gpu::GpuContext;

macro_rules! gpu_or_skip {
    () => {
        match GpuContext::headless() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("skipping: no GPU available ({e})");
                return;
            }
        }
    };
}

fn add(graph: &mut Graph, reg: &OpRegistry, parent: NodeId, op: &str, name: &str) -> NodeId {
    let def = reg.get(op).unwrap().clone();
    graph.create(parent, &def, Some(name)).unwrap()
}

#[test]
fn webserver_http_callback_responds() {
    let gpu = gpu_or_skip!();
    let reg = registry();
    let mut graph = Graph::new();
    let root = graph.root();
    let ws = add(&mut graph, &reg, root, "webserverDAT", "webserver1");
    graph.set_param(ws, "port", Value::Int(0)).unwrap();

    let mut engines = Engines::new(gpu);
    if engines.python_error().is_some() {
        return;
    }
    let mut cook = CookEngine::new();
    let mut time = CookContext::default();

    let mut port = None;
    for _ in 0..120 {
        time.frame += 1;
        engines.begin_frame();
        cook.cook_frame(&graph, &[ws], &time, &mut engines)
            .unwrap();
        engines.end_frame();
        if let Some(p) = engines
            .dat_data(ws)
            .and_then(|d| d.lookup("port", 1))
            .and_then(|p| p.parse().ok())
        {
            port = Some(p);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let port: u16 = port.expect("server did not bind");

    let handle = thread::spawn(move || {
        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        stream
            .write_all(b"GET /test HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut buf = Vec::new();
        stream.read_to_end(&mut buf).unwrap();
        String::from_utf8_lossy(&buf).into_owned()
    });

    for _ in 0..200 {
        time.frame += 1;
        engines.begin_frame();
        cook.cook_frame(&graph, &[ws], &time, &mut engines)
            .unwrap();
        engines.end_frame();
        if handle.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let body = handle.join().unwrap();
    assert!(body.contains("200"), "{body}");
    assert!(body.contains("hello from opentouchdesigner"), "{body}");
}
