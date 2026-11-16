fn channel_last(chans: &[(String, Vec<f32>)], key: &str) -> Option<f32> {
    chans
        .iter()
        .find(|(n, _)| n == key)
        .and_then(|(_, s)| s.last().copied())
}

fn movie_drop_from_fps(fps: f64) -> bool {
    (fps - 30000.0 / 1001.0).abs() < 0.05 || (fps - 29.97).abs() < 0.01
}

fn has_smpte_fields(chans: &[(String, Vec<f32>)]) -> bool {
    channel_last(chans, "hour").is_some()
        && channel_last(chans, "frame").is_some()
        && (channel_last(chans, "min").is_some() || channel_last(chans, "minute").is_some())
        && (channel_last(chans, "sec").is_some() || channel_last(chans, "second").is_some())
}

fn labels_to_total(hour: u32, min: u32, sec: u32, frame: u32, fps: f64, drop: bool) -> i64 {
    if drop {
        let nominal = ((hour as i64 * 60 + min as i64) * 60 + sec as i64) * 30 + frame as i64;
        let minutes = hour as i64 * 60 + min as i64;
        nominal - 2 * (minutes - minutes / 10)
    } else {
        let fps_i = fps.round().max(1.0) as i64;
        ((hour as i64 * 60 + min as i64) * 60 + sec as i64) * fps_i + frame as i64
    }
}

pub fn movie_timecode_frame_index(
    chans: &[(String, Vec<f32>)],
    movie_fps: f64,
    timecode_mode: &str,
) -> Option<i64> {
    let fps = movie_fps.max(1e-6);
    let drop = movie_drop_from_fps(fps);

    if has_smpte_fields(chans) {
        let hour = channel_last(chans, "hour")? as u32;
        let min = channel_last(chans, "min").or_else(|| channel_last(chans, "minute"))? as u32;
        let sec = channel_last(chans, "sec").or_else(|| channel_last(chans, "second"))? as u32;
        let frame = channel_last(chans, "frame")? as u32;
        let total = labels_to_total(hour, min, sec, frame, fps, drop);
        return Some(total);
    }

    if let Some(tf) = channel_last(chans, "total_frames") {
        return Some(tf.floor() as i64);
    }

    let v = chans.iter().find_map(|(_, s)| s.last()).copied()?;
    Some(if timecode_mode == "usefiletimecode" {
        v.floor() as i64
    } else {
        (f64::from(v) * fps).floor() as i64
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smpte_labels_drive_frame_index() {
        let chans = vec![
            ("hour".into(), vec![0.0]),
            ("min".into(), vec![0.0]),
            ("sec".into(), vec![2.0]),
            ("frame".into(), vec![0.0]),
        ];
        assert_eq!(movie_timecode_frame_index(&chans, 30.0, "absolute"), Some(60));
    }

    #[test]
    fn total_frames_channel() {
        let chans = vec![("total_frames".into(), vec![100.0])];
        assert_eq!(movie_timecode_frame_index(&chans, 24.0, "absolute"), Some(100));
    }
}