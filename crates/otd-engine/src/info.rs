use otd_chop::data::ChopData;
use otd_core::{CookEngine, Crossing, EvalContext, Family, Graph, NodeId};
use otd_dat::DatData;
use otd_gpu::TopEngine;
use otd_gpu::ops::{MOVIE_IN, VIDEO_DEVICE_IN};
use otd_sop::Geometry;

use crate::ChannelStore;
use crate::DatStore;
use crate::GeometryStore;

fn push(ch: &mut Vec<(String, f32)>, name: &str, value: f32) {
    ch.push((name.to_string(), value));
}

pub fn resolve_op_from(graph: &Graph, from: NodeId, path: &str) -> Option<NodeId> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        graph.find(path)
    } else {
        graph.find_from(from, path)
    }
}

fn cook_channels(cook: &CookEngine, id: NodeId, ch: &mut Vec<(String, f32)>) {
    push(ch, "total_cooks", cook.cook_count(id) as f32);
    push(ch, "cook", cook.cook_count(id) as f32);
    push(ch, "cook_time", cook.last_cook_us(id) as f32 / 1000.0);
    push(ch, "average_cook_time", cook.avg_cook_ms(id) as f32);
    push(ch, "cook_rate", cook.cook_rate(id) as f32);
    push(ch, "cpu_cook_time", cook.frame_cost_ms(id) as f32);
}

fn chop_info(data: &ChopData, ch: &mut Vec<(String, f32)>) {
    let n = data.num_samples().max(1);
    let rate = data.sample_rate.max(1e-6);
    push(ch, "num_channels", data.num_channels() as f32);
    push(ch, "num_samples", data.num_samples() as f32);
    push(ch, "sample_rate", rate as f32);
    push(ch, "start", 0.0);
    push(ch, "end", (n as f64 / rate) as f32);
    push(ch, "rate", rate as f32);
}

fn sop_info(geo: &Geometry, ch: &mut Vec<(String, f32)>) {
    push(ch, "num_points", geo.num_points() as f32);
    push(ch, "num_primitives", geo.num_primitives() as f32);
    push(ch, "num_vertices", geo.num_vertices() as f32);
}

fn dat_info(data: &DatData, ch: &mut Vec<(String, f32)>) {
    push(ch, "num_rows", data.num_rows() as f32);
    push(ch, "num_cols", data.num_cols() as f32);
}

pub fn operator_info_crossing(
    top: &TopEngine,
    cook: Option<&CookEngine>,
    graph: &Graph,
    channels: &ChannelStore,
    dats: &DatStore,
    geometry: &GeometryStore,
    info_id: NodeId,
    eval: &EvalContext,
) -> Option<Crossing> {
    let info = graph.get(info_id)?;
    let watch_path = info.param("op")?.eval(eval).as_str();
    let watch = resolve_op_from(graph, info_id, &watch_path)?;
    let watch_node = graph.get(watch)?;

    let mut ch = Vec::new();
    if let Some(cook) = cook {
        cook_channels(cook, watch, &mut ch);
    }

    let mut open = false;
    match watch_node.family {
        Family::Top => {
            if watch_node.op_type == MOVIE_IN || watch_node.op_type == VIDEO_DEVICE_IN {
                open = top.movie_info(watch, &mut ch);
            } else if let Some(out) = top.output_texture_size(watch) {
                push(&mut ch, "resx", out.0 as f32);
                push(&mut ch, "resy", out.1 as f32);
                open = true;
            }
        }
        Family::Chop => {
            if let Some(data) = channels.get(watch) {
                chop_info(data, &mut ch);
                open = true;
            }
        }
        Family::Sop => {
            if let Some(geo) = geometry.get(watch) {
                sop_info(geo, &mut ch);
                open = !geo.is_empty();
            }
        }
        Family::Dat => {
            if let Some(data) = dats.get(watch) {
                dat_info(data, &mut ch);
                open = !data.rows.is_empty();
            }
        }
        Family::Mat | Family::Comp => {}
    }

    if !ch.iter().any(|(n, _)| n == "open") {
        push(&mut ch, "open", if open { 1.0 } else { 0.0 });
    }

    Some(Crossing::Info { channels: ch })
}
