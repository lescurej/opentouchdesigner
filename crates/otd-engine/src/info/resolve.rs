use otd_chop::data::ChopData;
use otd_core::{CookEngine, Crossing, EvalContext, Family, Graph, NodeId};
use otd_dat::DatData;
use otd_gpu::TopEngine;
use otd_gpu::ops::{MOVIE_IN, VIDEO_DEVICE_IN};
use otd_sop::Geometry;

use crate::ChannelStore;
use crate::DatStore;
use crate::Engines;
use crate::GeometryStore;

pub(crate) fn push(ch: &mut Vec<(String, f32)>, name: &str, value: f32) {
    ch.push((name.to_string(), value));
}

pub fn converter_param_dependencies(graph: &Graph, id: NodeId) -> Vec<NodeId> {
    let Some(node) = graph.get(id) else {
        return Vec::new();
    };
    let path = |key: &str| -> Option<NodeId> {
        let raw = graph
            .node(id)
            .param(key)
            .map(|p| p.eval(&EvalContext::default()).as_str())
            .unwrap_or_default();
        resolve_op_from(graph, id, &raw)
    };
    match node.op_type.as_str() {
        "dattochopCHOP" => path("dat").into_iter().collect(),
        "soptochopCHOP" => path("sop").into_iter().collect(),
        "toptochopCHOP" | "blobtrackCHOP" => path("top").into_iter().collect(),
        "choptotopTOP" => path("chop").into_iter().collect(),
        "choptodatDAT" => path("chop").into_iter().collect(),
        "ltcoutCHOP" => path("timecodeop").into_iter().collect(),
        "moviefileinTOP" => {
            let play = graph
                .node(id)
                .param("playmode")
                .map(|p| p.eval(&EvalContext::default()).as_str())
                .unwrap_or_default();
            if play == "timecodeop" {
                path("timecodeop").into_iter().collect()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

pub fn crossing_from_converter_param(
    graph: &Graph,
    id: NodeId,
    top: &TopEngine,
    channels: &ChannelStore,
    dats: &DatStore,
    geometry: &GeometryStore,
) -> Option<Crossing> {
    let node = graph.get(id)?;
    let path = |key: &str| -> Option<NodeId> {
        let raw = graph
            .node(id)
            .param(key)
            .map(|p| p.eval(&EvalContext::default()).as_str())
            .unwrap_or_default();
        resolve_op_from(graph, id, &raw)
    };
    match node.op_type.as_str() {
        "dattochopCHOP" => {
            let src = path("dat")?;
            let dat = dats.get(src)?;
            if let Some(m) = &dat.chop_mirror {
                let chop_id = graph.find(&m.chop_path)?;
                let data = channels.get(chop_id)?;
                let names: Vec<String> = data.channels.iter().map(|c| c.name.clone()).collect();
                let samples: Vec<Vec<f32>> =
                    data.channels.iter().map(|c| c.copy_samples()).collect();
                let rows = otd_dat::chop_mirror::materialize_table(m, &names, &samples);
                return Some(Crossing::Table(rows));
            }
            Some(Crossing::Table(dat.rows.clone()))
        }
        "soptochopCHOP" => {
            let src = path("sop")?;
            let geo = geometry.get(src)?;
            let attrs = vec![
                ("P".to_string(), 3),
                ("N".to_string(), 3),
                ("uv".to_string(), 2),
                ("Cd".to_string(), 4),
            ];
            let mut data = Vec::with_capacity(geo.points.len() * 12);
            for p in &geo.points {
                data.extend_from_slice(&p.position);
                data.extend_from_slice(&p.normal);
                data.extend_from_slice(&p.uv);
                data.extend_from_slice(&p.color);
            }
            Some(Crossing::Points { attrs, data })
        }
        "toptochopCHOP" | "blobtrackCHOP" => {
            let src = path("top")?;
            let tex = top.output(graph, src)?;
            let (width, height, rgba) = otd_gpu::read_pixels_rgba_f32(top.context(), tex).ok()?;
            Some(Crossing::Pixels {
                width,
                height,
                rgba,
            })
        }
        "choptotopTOP" => {
            let src = path("chop")?;
            let data = channels.get(src)?;
            Some(Crossing::Channels {
                names: data.channels.iter().map(|c| c.name.clone()).collect(),
                samples: data.channels.iter().map(|c| c.copy_samples()).collect(),
                sample_rate: data.sample_rate,
            })
        }
        "choptodatDAT" => {
            let src = path("chop")?;
            let data = channels.get(src)?;
            Some(Crossing::Channels {
                names: data.channels.iter().map(|c| c.name.clone()).collect(),
                samples: data.channels.iter().map(|c| c.copy_samples()).collect(),
                sample_rate: data.sample_rate,
            })
        }
        "ltcoutCHOP" => ltc_timecode_crossing(graph, path("timecodeop")?, channels, dats),
        _ => None,
    }
}

fn ltc_timecode_crossing(
    graph: &Graph,
    src: NodeId,
    channels: &ChannelStore,
    dats: &DatStore,
) -> Option<Crossing> {
    match graph.get(src)?.family {
        Family::Dat => Some(Crossing::Table(dats.get(src)?.rows.clone())),
        Family::Chop => {
            let data = channels.get(src)?;
            Some(Crossing::Channels {
                names: data.channels.iter().map(|c| c.name.clone()).collect(),
                samples: data.channels.iter().map(|c| c.copy_samples()).collect(),
                sample_rate: data.sample_rate,
            })
        }
        _ => None,
    }
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

pub fn choptodat_chop_table_src(
    graph: &Graph,
    dat_id: NodeId,
    channels: &otd_chop::engine::ChannelStore,
    eval: &otd_core::EvalContext<'_>,
) -> Option<(String, usize, usize)> {
    if graph.get(dat_id)?.op_type != "choptodatDAT" {
        return None;
    }
    let raw = graph.node(dat_id).param("chop")?.eval(eval).as_str();
    if raw.is_empty() {
        return None;
    }
    let src = resolve_op_from(graph, dat_id, &raw)?;
    let data = channels.get(src)?;
    Some((graph.path(src), data.channels.len(), data.num_samples()))
}

pub fn resolve_chop_output_sources(graph: &Graph, from: NodeId, paths: &str) -> Vec<NodeId> {
    let mut out = Vec::new();
    for reference in paths.split_whitespace() {
        for id in graph.chops_matching_reference(from, reference) {
            let Some(src) = graph.resolve_output(id) else {
                continue;
            };
            if !out.contains(&src) {
                out.push(src);
            }
        }
    }
    out
}

pub(crate) fn cook_channels(cook: &CookEngine, id: NodeId, ch: &mut Vec<(String, f32)>) {
    push(ch, "total_cooks", cook.cook_count(id) as f32);
    push(ch, "cook", cook.cook_count(id) as f32);
    push(ch, "cook_time", cook.last_cook_us(id) as f32 / 1000.0);
    push(ch, "average_cook_time", cook.avg_cook_ms(id) as f32);
    push(ch, "cook_rate", cook.cook_rate(id) as f32);
    push(ch, "cpu_cook_time", cook.frame_cost_ms(id) as f32);
}

pub(crate) fn chop_info(data: &ChopData, ch: &mut Vec<(String, f32)>) {
    let n = data.num_samples().max(1);
    let rate = data.sample_rate.max(1e-6);
    push(ch, "num_channels", data.num_channels() as f32);
    push(ch, "num_samples", data.num_samples() as f32);
    push(ch, "sample_rate", rate as f32);
    push(ch, "start", 0.0);
    push(ch, "end", (n as f64 / rate) as f32);
    push(ch, "rate", rate as f32);
}

pub(crate) fn sop_info(geo: &Geometry, ch: &mut Vec<(String, f32)>) {
    push(ch, "num_points", geo.num_points() as f32);
    push(ch, "num_primitives", geo.num_primitives() as f32);
    push(ch, "num_vertices", geo.num_vertices() as f32);
}

pub(crate) fn dat_info(data: &DatData, ch: &mut Vec<(String, f32)>) {
    push(ch, "num_rows", data.num_rows() as f32);
    push(ch, "num_cols", data.num_cols() as f32);
}

pub(crate) fn push_cook_line(out: &mut Vec<String>, cook: &CookEngine, id: NodeId, frame: i64) {
    if !cook.has_cooked(id) {
        out.push("Cook: not cooked yet".into());
        return;
    }
    let last = cook.last_cook_us(id) as f64 / 1000.0;
    let avg = cook.avg_cook_ms(id);
    let rate = cook.cook_rate(id) * 100.0;
    let frame_note = if cook.cooked_this_frame(id, frame) {
        "ran this frame"
    } else {
        "cached this frame"
    };
    out.push(format!(
        "Cook: {last:.3} ms last · {avg:.3} ms avg · {rate:.0}% cooks · {frame_note}"
    ));
}

pub(crate) fn push_output_line(
    engines: &Engines,
    graph: &Graph,
    id: NodeId,
    family: Family,
    out: &mut Vec<String>,
) {
    match family {
        Family::Top => {
            if let Some(tex) = engines.top.output(graph, id) {
                let w = tex.key.width;
                let h = tex.key.height;
                let (format, bits, bytes_per_pixel) = top_pixel_format(tex.key.format);
                out.push(format!("Resolution: {w}×{h}"));
                out.push(format!("Aspect: {}", fmt_num(w as f64 / h.max(1) as f64)));
                out.push(format!("Pixel Format: {format}"));
                if bits > 0 {
                    out.push(format!("Depth: {bits}-bit"));
                }
                if bytes_per_pixel > 0 {
                    let mb = (w as f64) * (h as f64) * bytes_per_pixel as f64 / 1.0e6;
                    out.push(format!("GPU Memory: {} MB", fmt_num(mb)));
                }
            }
            let op = graph.node(id).op_type.as_str();
            if op == MOVIE_IN || op == VIDEO_DEVICE_IN {
                let mut ch = Vec::new();
                if engines.top.movie_info(id, &mut ch) {
                    let index = ch.iter().find(|(n, _)| n == "index").map(|(_, v)| *v);
                    let length = ch.iter().find(|(n, _)| n == "length").map(|(_, v)| *v);
                    if let (Some(i), Some(len)) = (index, length) {
                        out.push(format!("Movie: frame {:.0} / {:.0}", i, len));
                    }
                    if let Some(fps) = ch.iter().find(|(n, _)| n == "fps").map(|(_, v)| *v) {
                        if fps > 0.0 {
                            out.push(format!("Movie: {fps:.3} fps"));
                        }
                    }
                }
            }
        }
        Family::Chop => {
            let Some(data) = engines.channels.get(id) else {
                return;
            };
            let samples = data.num_samples();
            let rate = data.sample_rate;
            out.push(format!("Channels: {}", data.num_channels()));
            out.push(format!("Samples: {samples}"));
            out.push(format!("Sample Rate: {}", fmt_num(rate)));
            if data.time_sliced {
                out.push("Time Slice: on".into());
            }
        }
        Family::Sop => {
            let Some(geo) = engines.geometry.get(id) else {
                return;
            };
            if geo.is_empty() {
                out.push("Geometry: empty".into());
            } else {
                out.push(format!("Points: {}", geo.num_points()));
                out.push(format!("Primitives: {}", geo.num_primitives()));
                out.push(format!("Vertices: {}", geo.num_vertices()));
            }
        }
        Family::Dat => {
            let Some(data) = engines.dats.get(id) else {
                return;
            };
            if data.is_text {
                let text = data.as_text();
                let lines = if text.is_empty() {
                    0
                } else {
                    text.lines().count()
                };
                out.push(format!(
                    "Text: {lines} lines · {} characters",
                    text.chars().count()
                ));
            } else {
                out.push(format!("Rows: {}", data.num_rows()));
                out.push(format!("Columns: {}", data.num_cols()));
            }
        }
        Family::Comp => {
            let n = graph.children(id).len();
            let word = if n == 1 { "operator" } else { "operators" };
            out.push(format!("Network: {n} {word} inside"));
        }
        Family::Mat => {}
    }
}

fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    if (v - v.round()).abs() < 1e-3 {
        return format!("{}", v.round() as i64);
    }
    let s = format!("{v:.4}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn top_pixel_format(format: wgpu::TextureFormat) -> (&'static str, u32, u32) {
    use wgpu::TextureFormat::*;
    match format {
        Rgba8Unorm | Rgba8UnormSrgb | Bgra8Unorm | Bgra8UnormSrgb => ("8-bit fixed (RGBA)", 8, 4),
        Rgba16Unorm => ("16-bit fixed (RGBA)", 16, 8),
        Rgba16Float => ("16-bit float (RGBA)", 16, 8),
        Rgba32Float => ("32-bit float (RGBA)", 32, 16),
        Rgb10a2Unorm => ("10-bit RGB 2-bit A", 10, 4),
        Rg11b10Ufloat => ("11-bit float (RGB)", 11, 4),
        _ => ("custom", 0, 0),
    }
}

