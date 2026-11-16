/// ffmpeg / WAV order: left, right, center, LFE, back left, back right,
/// side left, side right.
pub(crate) fn channel_names(n: usize) -> Vec<String> {
    const NAMED: [&str; 8] = [
        "left",
        "right",
        "center",
        "lfe",
        "backleft",
        "backright",
        "sideleft",
        "sideright",
    ];
    match n {
        1 => vec!["chan1".into()],
        2 => vec!["left".into(), "right".into()],
        6 | 8 => NAMED[..n].iter().map(|s| (*s).to_string()).collect(),
        _ => (1..=n).map(|i| format!("chan{i}")).collect(),
    }
}

/// Place one source frame onto the device frame.
pub(crate) fn write_speakers(src: &[f32], frame: &mut [f32]) {
    if src.is_empty() || frame.is_empty() {
        frame.fill(0.0);
        return;
    }
    if src.len() == 1 {
        frame.fill(src[0]);
        return;
    }
    if frame.len() >= src.len() {
        frame[..src.len()].copy_from_slice(src);
        frame[src.len()..].fill(0.0);
        return;
    }
    if frame.len() == 1 {
        frame[0] = src.iter().sum::<f32>() / src.len() as f32;
        return;
    }
    // Stereo (or any device smaller than the file): fold into left / right.
    // Center and LFE go to both; backs and sides stay on their side.
    let g = |i: usize| src.get(i).copied().unwrap_or(0.0);
    let c = 0.707;
    let mut left = g(0);
    let mut right = g(1);
    if src.len() >= 3 {
        left += c * g(2);
        right += c * g(2);
    }
    if src.len() >= 4 {
        left += 0.5 * g(3);
        right += 0.5 * g(3);
    }
    if src.len() >= 5 {
        left += c * g(4);
    }
    if src.len() >= 6 {
        right += c * g(5);
    }
    if src.len() >= 7 {
        left += c * g(6);
    }
    if src.len() >= 8 {
        right += c * g(7);
    }
    frame[0] = left;
    frame[1] = right;
    frame[2..].fill(0.0);
}
