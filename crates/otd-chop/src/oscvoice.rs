#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PushDecision {
    Append,
    Skip,
    Seek,
}

pub(crate) fn push_decision(last_end: f64, end: f64) -> PushDecision {
    if !last_end.is_finite() {
        return PushDecision::Append;
    }
    if end + 1e-4 < last_end {
        return PushDecision::Seek;
    }
    if (end - last_end).abs() < 1e-8 {
        return PushDecision::Skip;
    }
    PushDecision::Append
}
