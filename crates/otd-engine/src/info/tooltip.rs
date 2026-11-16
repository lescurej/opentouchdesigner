use otd_core::{CookEngine, Crossing, EvalContext, Family, Graph, MovieAudio, Node, NodeId};
use otd_gpu::TopEngine;
use otd_gpu::ops::{MOVIE_IN, VIDEO_DEVICE_IN};

use crate::ChannelStore;
use crate::DatStore;
use crate::Engines;
use crate::GeometryStore;

use super::resolve::{chop_info, cook_channels, dat_info, push, push_cook_line, push_output_line, resolve_op_from, sop_info};

fn push_flag_line(node: &Node, out: &mut Vec<String>) {
    let mut flags = Vec::new();
    if node.flags.bypass {
        flags.push("bypass");
    }
    if node.flags.lock {
        flags.push("lock");
    }
    if node.flags.clone_immune {
        flags.push("clone immune");
    }
    if !flags.is_empty() {
        out.push(format!("Flags: {}", flags.join(" · ")));
    }
}

/// Multi-line text for the network editor node hover tooltip.
pub fn node_infotip(
    engines: &Engines,
    cook: &CookEngine,
    graph: &Graph,
    id: NodeId,
    frame: i64,
    summary: &str,
) -> String {
    let node = graph.node(id);
    let path = graph.path(id);
    let mut lines = vec![format!("{} · {}", node.op_type, node.name)];
    if !summary.is_empty() {
        lines.push(summary.to_string());
    }
    lines.push(path);
    if let Some(master) = &node.clone_of {
        lines.push(format!("Clone of: {master}"));
    } else if let Some(file) = &node.external {
        lines.push(format!("External: {file}"));
    }

    push_cook_line(&mut lines, cook, id, frame);
    if cook.is_time_dependent(id) {
        lines.push("Time-dependent".into());
    }

    let inputs = node.inputs.len();
    if inputs > 0 {
        let wired = node.inputs.iter().filter(|s| s.is_some()).count();
        lines.push(format!("Inputs: {wired}/{inputs} wired"));
    }

    push_output_line(engines, graph, id, node.family, &mut lines);
    push_flag_line(node, &mut lines);

    if let Some(msg) = engines.node_message(graph, id) {
        lines.push(msg.text);
    } else if let Some(status) = engines.top.status(id) {
        lines.push(status.to_string());
    }

    lines.join("\n")
}

#[allow(clippy::too_many_arguments)]
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

pub fn audiomovie_playback_crossing(
    top: &TopEngine,
    graph: &Graph,
    from: NodeId,
    eval: &EvalContext,
) -> Option<Crossing> {
    let path = graph
        .node(from)
        .param("top")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default();
    let watch = resolve_op_from(graph, from, &path)?;
    let movie = graph.node(watch);
    if movie.op_type != MOVIE_IN {
        return None;
    }
    let mut ch = Vec::new();
    if top.movie_info(watch, &mut ch) {
        return Some(Crossing::Info { channels: ch });
    }
    let speed = movie
        .param("speed")
        .map(|p| p.eval(eval).as_f64())
        .unwrap_or(1.0);
    let fps = movie
        .param("rate")
        .map(|p| p.eval(eval).as_f64())
        .unwrap_or(60.0)
        .max(1e-6);
    let playhead = eval.time * speed * fps;
    push(&mut ch, "playhead", playhead as f32);
    push(&mut ch, "index", playhead as f32);
    push(&mut ch, "sample_rate", fps as f32);
    push(&mut ch, "open", 0.0);
    Some(Crossing::Info { channels: ch })
}

/// PCM from the movie reader's audio track, on the picture timeline.
pub fn audiomovie_soundtrack_crossing(
    top: &TopEngine,
    graph: &Graph,
    from: NodeId,
    eval: &EvalContext,
) -> Option<Crossing> {
    let path = graph
        .node(from)
        .param("top")
        .map(|p| p.eval(eval).as_str())
        .unwrap_or_default();
    let watch = resolve_op_from(graph, from, &path)?;
    if graph.node(watch).op_type != MOVIE_IN {
        return None;
    }
    let audio = top.movie_audio(watch).unwrap_or(MovieAudio::Pending);
    Some(Crossing::MovieAudio(audio))
}
