/// A channel name has to survive being typed into an expression.
pub(crate) fn sanitise(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    if cleaned.starts_with(|ch: char| ch.is_ascii_digit()) {
        format!("_{cleaned}")
    } else if cleaned.is_empty() {
        "chan".into()
    } else {
        cleaned
    }
}

