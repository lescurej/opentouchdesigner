use std::sync::{Arc, Mutex};

use otd_core::{CookContext, MovieAudio, MoviePcm};

use crate::data::{Channel, ChopData, slice_len};
use crate::ops::ChopCtx;

use super::transport::{AudioMovieState, MovieTransport};

pub(crate) fn params() -> otd_core::indexmap::IndexMap<String, otd_core::Param> {
    use otd_core::Param;
    let mut m = otd_core::indexmap::IndexMap::new();
    m.insert(
        "top".into(),
        Param::str("").with_label("Movie File In TOP").as_path_ref(),
    );
    m.insert(
        "volume".into(),
        Param::float(1.0).with_label("Volume").with_range(0.0, 8.0),
    );
    m
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let chop_path = c.path.to_string();
    let volume = c
        .node
        .param("volume")
        .map(|p| p.eval(c.eval).as_f32())
        .unwrap_or(1.0);
    let silent = |ctx: &CookContext| {
        let n = slice_len(48_000.0, ctx);
        ChopData::new(vec![Channel::constant("chan1", 0.0, n)], 48_000.0, true)
    };

    let Some(movie_id) = resolve_movie(c) else {
        c.io.note(&chop_path, "Set Movie File In TOP");
        c.io.audiomovie.remove(&chop_path);
        return silent(c.time);
    };
    let Some(movie) = c.graph.get(movie_id) else {
        c.io.note(&chop_path, "Movie TOP not found");
        return silent(c.time);
    };
    if movie.op_type != "moviefileinTOP" {
        c.io.note(&chop_path, "TOP must be a Movie File In");
        return silent(c.time);
    }

    let file = movie
        .param("file")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    if file.trim().is_empty() {
        c.io.audiomovie.remove(&chop_path);
        c.io.clear_note(&chop_path);
        return silent(c.time);
    }

    let speed = movie
        .param("speed")
        .map(|p| p.eval(c.eval).as_f64())
        .unwrap_or(1.0);
    let movie_play = movie
        .param("play")
        .map(|p| p.eval(c.eval).as_bool())
        .unwrap_or(true);
    let soundtrack = c
        .foreign
        .iter()
        .flatten()
        .find_map(|x| x.as_movie_audio())
        .cloned();

    match &soundtrack {
        Some(MovieAudio::Ready(pcm)) if pcm.frames() > 0 && pcm.channels > 0 => {
            adopt_soundtrack(c, &chop_path, &file, pcm);
        }
        Some(MovieAudio::Missing) => {
            c.io.audiomovie.remove(&chop_path);
            c.io.note(&chop_path, "Movie has no audio track");
            return silent(c.time);
        }
        Some(MovieAudio::Failed(message)) => {
            c.io.audiomovie.remove(&chop_path);
            c.io.note(&chop_path, message.clone());
            return silent(c.time);
        }
        Some(MovieAudio::Pending) | Some(MovieAudio::Ready(_)) | None => {
            if c.io
                .audiomovie
                .get(&chop_path)
                .is_some_and(|s| s.file != file)
            {
                c.io.audiomovie.remove(&chop_path);
            }
            c.io.note(&chop_path, "Reading soundtrack");
            if !c.io.audiomovie.contains_key(&chop_path) {
                return silent(c.time);
            }
        }
    }
    c.io.clear_note(&chop_path);

    let movie_fps_param = movie
        .param("rate")
        .map(|p| p.eval(c.eval).as_f64())
        .unwrap_or(60.0)
        .max(1e-6);
    let info = c.foreign.iter().flatten().find_map(|x| x.as_info());
    let (playhead_frames, movie_fps) = if let Some(info) = info {
        let ph = info
            .iter()
            .find(|(n, _)| n == "playhead")
            .or_else(|| info.iter().find(|(n, _)| n == "index"))
            .map(|(_, v)| *v)
            .unwrap_or(0.0) as f64;
        let fps = info
            .iter()
            .find(|(n, _)| n == "sample_rate")
            .map(|(_, v)| *v)
            .filter(|f| *f > 0.0)
            .map(f64::from)
            .unwrap_or(movie_fps_param);
        (ph, fps)
    } else {
        (c.time.time * speed * movie_fps_param, movie_fps_param)
    };
    let playmode = movie
        .param("playmode")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    let media_sec = match playmode.as_str() {
        "sequential" | "specify" => (playhead_frames / movie_fps.max(1e-6)).max(0.0),
        _ => (c.time.time * speed).max(0.0),
    };

    let state = c.io.audiomovie.get(&chop_path).unwrap();
    let transport = state.transport.clone();
    let mut transport = transport.lock().unwrap_or_else(|e| e.into_inner());
    if transport.pcm.frames() == 0 {
        return silent(c.time);
    }
    transport.sync(media_sec, speed, movie_play, volume);
    transport.slice(c.time)
}

fn adopt_soundtrack(c: &mut ChopCtx, chop_path: &str, file: &str, pcm: &Arc<MoviePcm>) {
    let pcm_id = Arc::as_ptr(pcm) as usize;
    let same =
        c.io.audiomovie
            .get(chop_path)
            .is_some_and(|s| s.file == file && s.pcm_id == pcm_id);
    if same {
        return;
    }
    c.io.audiomovie.insert(
        chop_path.to_string(),
        AudioMovieState {
            file: file.to_string(),
            pcm_id,
            transport: Arc::new(Mutex::new(MovieTransport::from_pcm(Arc::clone(pcm)))),
        },
    );
}

fn resolve_movie(c: &ChopCtx<'_>) -> Option<otd_core::NodeId> {
    let path = c.s("top");
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

/// Input of an Audio Device Out, following nulls, when it is an Audio Movie.
pub(crate) fn transport_for_input(
    graph: &otd_core::Graph,
    input: Option<otd_core::NodeId>,
    io: &crate::io::Io,
) -> Option<Arc<Mutex<MovieTransport>>> {
    let mut id = input?;
    for _ in 0..8 {
        let node = graph.get(id)?;
        if node.op_type == crate::io::AUDIO_MOVIE {
            let path = graph.path(id);
            return io.audiomovie.get(&path).map(|s| s.transport.clone());
        }
        if node.op_type == "nullCHOP" {
            id = node.inputs.first().copied().flatten()?;
            continue;
        }
        return None;
    }
    None
}
