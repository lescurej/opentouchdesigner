use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, MAX_SLICE, slice_len};
use crate::ops::ChopCtx;
use crate::timecode::{self, LtcStream, Smpte};

use super::{spec_animated, ChopSpec};

pub(crate) fn ltc_clock(menu: &str, drop_flag: bool) -> (f64, bool) {
    let (fps, can_drop) = match menu {
        "24" => (24.0f64, false),
        "25" => (25.0, false),
        "29.97" => (30000.0 / 1001.0, true),
        "60" => (60.0, false),
        _ => (30.0, true),
    };
    let drop = menu == "29.97" || (drop_flag && can_drop);
    let fps = if drop && (fps - 30.0).abs() < 0.05 {
        30000.0 / 1001.0
    } else {
        fps
    };
    (fps, drop)
}

pub(crate) fn params() -> IndexMap<String, Param> {
    let page = "LTC";
    let mut m = params! {
        "playmode" => Param::menu("sequential", &["sequential", "timecodeop"])
            .with_label("Play Mode")
            .with_page(page),
        "play" => Param::bool(true).with_label("Play").with_page(page),
        "cue" => Param::bool(false).with_label("Cue").with_page(page),
        "cuepulse" => Param::bool(false).with_label("Cue Pulse").with_page(page),
        "frame" => Param::int(0).with_label("Frame").with_range(0.0, 59.0).with_page(page),
        "second" => Param::int(0).with_label("Second").with_range(0.0, 59.0).with_page(page),
        "minute" => Param::int(0).with_label("Minute").with_range(0.0, 59.0).with_page(page),
        "hour" => Param::int(0).with_label("Hour").with_range(0.0, 23.0).with_page(page),
        "framerate" => Param::menu("30", &["24", "25", "29.97", "30", "60"])
            .with_label("Frame Rate")
            .with_page(page),
        "dropframe" => Param::bool(false).with_label("Drop Frame Numbering").with_page(page),
        "highfpsbehaviour" => Param::menu("cycleframes", &["cycleframes", "duplicateframes"])
            .with_label("High FPS Behaviour")
            .with_page(page),
        "timecodeop" => Param::str("")
            .with_label("Timecode Object/CHOP/DAT")
            .as_path_ref()
            .with_page(page),
        "audiorate" => Param::float(48000.0)
            .with_label("Audio Rate")
            .with_range(8000.0, 192000.0)
            .with_page(page),
        "level" => Param::float(0.5).with_label("Level").with_range(0.0, 1.0).with_page(page),
    };
    for i in 1..=8 {
        m.insert(
            format!("user{i}"),
            Param::int(0)
                .with_label(&format!("User Data {i}"))
                .with_range(0.0, 15.0)
                .with_page("User"),
        );
    }
    m
}

fn resolve_path(c: &ChopCtx, path: &str) -> Option<otd_core::NodeId> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        c.graph.find(path)
    } else {
        c.graph.find_from(c.id, path)
    }
}

fn named_last(names: &[String], samples: &[Vec<f32>], key: &str) -> Option<f32> {
    let i = names.iter().position(|n| n == key)?;
    samples.get(i)?.last().copied()
}

fn smpte_from_named(names: &[String], samples: &[Vec<f32>], fps: f64, drop: bool) -> Option<Smpte> {
    let hour = named_last(names, samples, "hour")?;
    let min = named_last(names, samples, "minute").or_else(|| named_last(names, samples, "min"))?;
    let sec = named_last(names, samples, "second").or_else(|| named_last(names, samples, "sec"))?;
    let frame = named_last(names, samples, "frame")?;
    let frac = named_last(names, samples, "frame_frac").unwrap_or(0.0) as f64;
    Some(timecode::smpte_from_labels(
        hour.max(0.0) as u32,
        min.max(0.0) as u32,
        sec.max(0.0) as u32,
        frame.max(0.0) as u32,
        frac,
        fps,
        drop,
    ))
}

fn referenced_smpte(c: &ChopCtx, fps: f64, drop: bool) -> Option<Smpte> {
    if let Some(cross) = c.foreign(0) {
        if let Some(rows) = cross.as_table() {
            let cell = rows
                .first()
                .and_then(|row| row.first())
                .map(|s| s.as_str())
                .unwrap_or("");
            return timecode::parse_timecode(cell, fps, drop);
        }
        if let Some((names, samples, _)) = cross.as_channels() {
            return smpte_from_named(names, samples, fps, drop);
        }
    }
    let id = resolve_path(c, &c.s("timecodeop"))?;
    smpte_from_chop(c.store.get(id)?, fps, drop)
}

fn smpte_from_chop(data: &ChopData, fps: f64, drop: bool) -> Option<Smpte> {
    let hour = data.value("hour")?;
    let min = data.value("minute").or_else(|| data.value("min"))?;
    let sec = data.value("second").or_else(|| data.value("sec"))?;
    let frame = data.value("frame")?;
    let frac = data.value("frame_frac").unwrap_or(0.0) as f64;
    Some(timecode::smpte_from_labels(
        hour.max(0.0) as u32,
        min.max(0.0) as u32,
        sec.max(0.0) as u32,
        frame.max(0.0) as u32,
        frac,
        fps,
        drop,
    ))
}

fn cue_frame_index(c: &ChopCtx, fps: f64, drop: bool) -> i64 {
    timecode::smpte_from_labels(
        c.i("hour").clamp(0, 23) as u32,
        c.i("minute").clamp(0, 59) as u32,
        c.i("second").clamp(0, 59) as u32,
        c.i("frame").max(0) as u32,
        0.0,
        fps,
        drop,
    )
    .total_frames
}

fn user_bits(c: &ChopCtx) -> [u8; 8] {
    let mut user = [0u8; 8];
    for (i, slot) in user.iter_mut().enumerate() {
        *slot = c.i(&format!("user{}", i + 1)).clamp(0, 15) as u8;
    }
    user
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let (fps, drop) = ltc_clock(&c.s("framerate"), c.b("dropframe"));
    let mut rate = c.f("audiorate") as f64;
    if rate < fps * 320.0 {
        rate = 48000.0;
    }
    let level = c.f("level").clamp(0.0, 1.0);
    let duplicate = c.s("highfpsbehaviour") == "duplicateframes";
    c.state.fit(4);
    let first = c.state.i[1] == 0;
    let rate_changed = (c.state.f[0] as f64 - rate).abs() > 1.0;
    let cursor_ready = c.state.i[1] == 1 && !rate_changed;
    let end = (c.time.time * rate).round() as i64;
    let nominal = slice_len(rate, c.time) as i64;
    let max_gap = (rate * 0.5) as i64;
    let start = if cursor_ready {
        let prev = c.state.i[0];
        if prev > end || end - prev > max_gap {
            (end - nominal).max(0)
        } else {
            prev.max(0)
        }
    } else {
        (end - nominal).max(0)
    };

    let follow = c.s("playmode") == "timecodeop";
    let cue = c.b("cue");
    let pulse = c.b("cuepulse") && c.state.prev[0] < 0.5;
    c.state.prev[0] = if c.b("cuepulse") { 1.0 } else { 0.0 };
    let was_cue = c.state.i[3] == 1;
    if first {
        c.state.i[2] = cue_frame_index(c, fps, drop);
        c.state.f[1] = 0.0;
    }
    let hold = if follow {
        if let Some(smpte) = referenced_smpte(c, fps, drop) {
            if first || smpte.total_frames != c.state.i[2] {
                c.state.i[2] = smpte.total_frames;
                c.state.f[1] = smpte.frame_frac as f32;
            }
            c.io.clear_note(c.path);
        } else {
            c.io.note(c.path, "timecodeop needs a CHOP or DAT");
        }
        true
    } else {
        let cue_at = cue_frame_index(c, fps, drop);
        if pulse || (cue && !was_cue) || (cue && cue_at != c.state.i[2]) {
            c.state.i[2] = cue_at;
            c.state.f[1] = 0.0;
        }
        c.io.clear_note(c.path);
        cue || !c.b("play")
    };
    c.state.i[3] = if cue { 1 } else { 0 };

    let stream = LtcStream {
        fps,
        drop_frame: drop,
        level,
        duplicate_high_fps: duplicate,
        user: user_bits(c),
    };
    let step = fps / rate.max(1.0);
    let frame_pos = c.state.i[2] as f64 + c.state.f[1] as f64;
    let (samples, next, advance) = if end > start {
        let n = ((end - start) as usize).min(MAX_SLICE);
        (
            timecode::ltc_samples_from_clock(frame_pos, n, step, hold, &stream),
            start + n as i64,
            n,
        )
    } else {
        (
            timecode::ltc_samples_from_clock(frame_pos, 1, step, hold, &stream),
            c.state.i[0].max(0),
            0,
        )
    };
    if advance > 0 {
        if hold {
            let mut frac = c.state.f[1] as f64 + advance as f64 * step;
            frac = frac.fract();
            if frac < 0.0 {
                frac += 1.0;
            }
            c.state.f[1] = frac as f32;
        } else {
            let pos = c.state.i[2] as f64 + c.state.f[1] as f64 + advance as f64 * step;
            c.state.i[2] = pos.floor() as i64;
            c.state.f[1] = pos.fract() as f32;
        }
    }
    c.state.i[0] = next;
    c.state.i[1] = 1;
    c.state.f[0] = rate as f32;
    ChopData::new(vec![Channel::new("ltc", samples)], rate, true)
}

pub const TYPE_NAME: &str = "ltcoutCHOP";

pub fn spec() -> ChopSpec {
    spec_animated(
        TYPE_NAME,
        "LTC Out",
        &[],
        "Encode the timeline as linear timecode audio.",
        params,
        cook,
    )
}
