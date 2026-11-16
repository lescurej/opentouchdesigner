use otd_core::movie_timecode_frame_index;
use otd_core::{EvalContext, Node};

use crate::scene::Scene;

pub(super) fn movie_cue_frame(node: &Node, eval: &EvalContext, fps: f64, length: i64) -> i64 {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    let val = get("cuepoint").map(|v| v.as_f64()).unwrap_or(0.0);
    let unit = get("cuepointunit")
        .map(|v| v.as_str())
        .unwrap_or_else(|| "indices".into());
    crate::video::position_from_units(val, &unit, fps, length)
}

pub(super) fn movie_length_frames(info: &crate::video::Info, fps: f64) -> i64 {
    (info.duration * fps).round().max(1.0) as i64
}

pub(super) fn movie_effective_fps(node: &Node, eval: &EvalContext, info: &crate::video::Info) -> f64 {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    if get("overridesample").map(|v| v.as_bool()).unwrap_or(false) {
        get("samplerate")
            .map(|v| v.as_f64())
            .filter(|f| *f > 0.0)
            .unwrap_or(info.fps)
    } else {
        info.fps
    }
    .max(1e-6)
}

pub(super) fn movie_trim_range(node: &Node, eval: &EvalContext, fps: f64, length: i64) -> (i64, i64) {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    if !get("trim").map(|v| v.as_bool()).unwrap_or(false) {
        return (0, length.saturating_sub(1).max(0));
    }
    let start_unit = get("tstartunit")
        .map(|v| v.as_str())
        .unwrap_or_else(|| "indices".into());
    let end_unit = get("tendunit")
        .map(|v| v.as_str())
        .unwrap_or_else(|| "fraction".into());
    let start = crate::video::position_from_units(
        get("tstart").map(|v| v.as_f64()).unwrap_or(0.0),
        &start_unit,
        fps,
        length,
    );
    let end = crate::video::position_from_units(
        get("tend").map(|v| v.as_f64()).unwrap_or(1.0),
        &end_unit,
        fps,
        length,
    );
    (
        start.max(0),
        end.max(start).min(length.saturating_sub(1).max(0)),
    )
}

pub(super) fn movie_map_index(
    index: i64,
    trim_start: i64,
    trim_end: i64,
    extend_left: &str,
    extend_right: &str,
) -> (i64, bool) {
    let span = (trim_end - trim_start + 1).max(1);
    let right = extend_right;
    let once_past_end = matches!(extend_right, "black" | "zero");

    if index < trim_start {
        let mapped = match extend_left {
            "cycle" => trim_start + (trim_start - index).rem_euclid(span),
            "black" | "zero" => return (trim_start, true),
            _ => trim_start,
        };
        return (mapped, false);
    }
    if index > trim_end {
        if once_past_end {
            return (trim_end, true);
        }
        let mapped = match right {
            "cycle" => trim_start + (index - trim_start).rem_euclid(span),
            "mirror" => {
                let o = index - trim_end;
                trim_end - o.rem_euclid(span)
            }
            "black" | "zero" => return (trim_end, true),
            _ => trim_end,
        };
        return (mapped, false);
    }
    (index, false)
}

pub(super) fn movie_decode_tuning(node: &Node, eval: &EvalContext) -> (bool, usize, u32) {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    let hw = get("hwdecode").map(|v| v.as_bool()).unwrap_or(true);
    let pre = get("prereadframes")
        .map(|v| v.as_i64())
        .unwrap_or(16)
        .clamp(0, 120) as usize;
    let cpus = get("maxdecodecpus")
        .map(|v| v.as_i64())
        .unwrap_or(0)
        .clamp(0, 64) as u32;
    (hw, pre.max(1), cpus)
}

pub(super) fn movie_open_start_index(node: &Node, eval: &EvalContext, info: &crate::video::Info) -> i64 {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    let speed = get("speed").map(|v| v.as_f64()).unwrap_or(1.0);
    let fps = movie_effective_fps(node, eval, info);
    let length = (info.duration * fps).round().max(1.0) as i64;
    let playmode = get("playmode")
        .map(|v| v.as_str())
        .unwrap_or_else(|| "sequential".into());
    match playmode.as_str() {
        "specify" => {
            let val = get("index").map(|v| v.as_f64()).unwrap_or(0.0);
            let unit = get("indexunit")
                .map(|v| v.as_str())
                .unwrap_or_else(|| "indices".into());
            crate::video::position_from_units(val, &unit, fps, length)
        }
        "sequential" => 0,
        _ => (eval.time * speed * fps).floor().max(0.0) as i64,
    }
}

pub(super) fn movie_specify_index(node: &Node, eval: &EvalContext, fps: f64, length: i64) -> i64 {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    let unit = get("indexunit")
        .map(|v| v.as_str())
        .unwrap_or_else(|| "indices".into());
    let val = get("index").map(|v| v.as_f64()).unwrap_or(0.0);
    crate::video::position_from_units(val, &unit, fps, length)
}

pub(super) fn movie_preread_frames(playmode: &str, user: usize) -> usize {
    if playmode == "specify" { 1 } else { user }
}

pub(super) fn movie_timecode_index(
    node: &Node,
    eval: &EvalContext,
    scene: Option<&dyn Scene>,
    fps: f64,
    length: i64,
    fallback: i64,
) -> i64 {
    let get = |key: &str| node.param(key).map(|p| p.eval(eval));
    let path = get("timecodeop").map(|v| v.as_str()).unwrap_or_default();
    if path.trim().is_empty() {
        return fallback;
    }
    let Some(chans) = scene.and_then(|s| s.channels(path.trim())) else {
        return fallback;
    };
    let mode = get("timecodemode")
        .map(|v| v.as_str())
        .unwrap_or_else(|| "absolute".into());
    let Some(idx) = movie_timecode_frame_index(&chans, fps, &mode) else {
        return fallback;
    };
    idx.clamp(0, length.saturating_sub(1).max(0))
}
