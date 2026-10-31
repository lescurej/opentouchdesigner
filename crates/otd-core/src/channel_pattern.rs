#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberedPattern<'a> {
    pub prefix: &'a str,
    pub start: i64,
    pub end: Option<i64>,
    pub suffix: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComponentPattern<'a> {
    pub prefix: &'a str,
    pub components: &'a str,
    pub suffix: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NamePattern<'a> {
    Numbered(NumberedPattern<'a>),
    Components(ComponentPattern<'a>),
}

fn parse_range_inner(inner: &str) -> (i64, Option<i64>) {
    if let Some((lo, hi)) = inner.split_once('-') {
        let start = lo.trim().parse().unwrap_or(1);
        let hi = hi.trim();
        if hi.is_empty() {
            (start, None)
        } else {
            let end = hi.parse().unwrap_or(start);
            (start, Some(end.max(start)))
        }
    } else {
        let n = inner.parse().unwrap_or(1);
        (n, Some(n))
    }
}

fn parse_name_pattern(template: &str) -> Option<NamePattern<'_>> {
    let open = template.find('[')?;
    let close = template[open + 1..].find(']')? + open + 1;
    if close <= open + 1 {
        return None;
    }
    let prefix = &template[..open];
    let inner = template[open + 1..close].trim();
    let suffix = &template[close + 1..];
    if inner.is_empty() {
        return None;
    }
    if inner.contains('-') {
        let (start, end) = parse_range_inner(inner);
        return Some(NamePattern::Numbered(NumberedPattern {
            prefix,
            start,
            end,
            suffix,
        }));
    }
    if inner.chars().all(|c| c.is_ascii_alphabetic()) {
        return Some(NamePattern::Components(ComponentPattern {
            prefix,
            components: inner,
            suffix,
        }));
    }
    if let Ok(n) = inner.parse::<i64>() {
        return Some(NamePattern::Numbered(NumberedPattern {
            prefix,
            start: n,
            end: Some(n),
            suffix,
        }));
    }
    None
}

pub fn parse_numbered_pattern(template: &str) -> Option<NumberedPattern<'_>> {
    match parse_name_pattern(template)? {
        NamePattern::Numbered(p) => Some(p),
        NamePattern::Components(_) => None,
    }
}

pub fn parse_component_pattern(template: &str) -> Option<ComponentPattern<'_>> {
    match parse_name_pattern(template)? {
        NamePattern::Components(p) => Some(p),
        NamePattern::Numbered(_) => None,
    }
}

pub fn channel_name_at(pattern: &NumberedPattern<'_>, index: usize) -> String {
    let n = pattern.start.saturating_add(index as i64);
    format!("{}{}{}", pattern.prefix, n, pattern.suffix)
}

fn component_name_at(pattern: &ComponentPattern<'_>, index: usize) -> Option<String> {
    let c = pattern.components.chars().nth(index)?;
    Some(format!("{}{}{}", pattern.prefix, c, pattern.suffix))
}

fn expand_component_pattern(pattern: &ComponentPattern<'_>) -> Vec<String> {
    pattern
        .components
        .chars()
        .map(|c| format!("{}{}{}", pattern.prefix, c, pattern.suffix))
        .collect()
}

fn numbered_channel_count(pattern: &NumberedPattern<'_>) -> usize {
    match pattern.end {
        Some(end) => ((end - pattern.start + 1).max(1)) as usize,
        None => 1,
    }
}

fn name_tokens(template: &str) -> Vec<&str> {
    template
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect()
}

fn channel_count_from_token(token: &str) -> usize {
    match parse_name_pattern(token.trim()) {
        Some(NamePattern::Numbered(p)) => numbered_channel_count(&p),
        Some(NamePattern::Components(p)) => p.components.chars().count(),
        None => 1,
    }
}

pub fn channel_count_from_name(template: &str) -> usize {
    let tokens = name_tokens(template);
    if tokens.is_empty() {
        return 1;
    }
    if tokens.len() > 1 {
        return tokens.iter().map(|t| channel_count_from_token(t)).sum();
    }
    channel_count_from_token(tokens[0])
}

fn expand_single_token(token: &str, count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let token = token.trim();
    if let Some(NamePattern::Components(p)) = parse_name_pattern(token) {
        return expand_component_pattern(&p);
    }
    if let Some(NamePattern::Numbered(pat)) = parse_name_pattern(token) {
        return (0..count)
            .map(|i| channel_name_at(&pat, i))
            .collect();
    }
    if count == 1 {
        vec![token.to_string()]
    } else {
        (0..count)
            .map(|i| format!("{token}{}", i + 1))
            .collect()
    }
}

pub fn expand_channel_names(template: &str, count: usize) -> Vec<String> {
    let tokens = name_tokens(template);
    if tokens.is_empty() {
        return Vec::new();
    }
    if tokens.len() > 1 {
        let mut out = Vec::new();
        for token in tokens {
            let n = channel_count_from_token(token);
            out.extend(expand_single_token(token, n));
        }
        return out;
    }
    expand_single_token(tokens[0], count)
}

pub fn expand_channel_name_field(field: &str) -> Vec<String> {
    let field = field.trim();
    if field.is_empty() {
        return vec!["chan1".to_string()];
    }
    let mut out = Vec::new();
    for token in field.split_whitespace() {
        if token.is_empty() {
            continue;
        }
        out.extend(expand_single_token(token, channel_count_from_token(token)));
    }
    if out.is_empty() {
        out.push("chan1".into());
    }
    out
}

pub fn channel_seed_for_name(base: i64, name: &str, index: usize) -> i64 {
    let mut h = base.wrapping_mul(7919);
    for b in name.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as i64);
    }
    h.wrapping_add(index as i64 * 104_729)
}

pub fn numbered_pattern_matches(pattern: &NumberedPattern<'_>, channel: &str) -> bool {
    if !channel.starts_with(pattern.prefix) || !channel.ends_with(pattern.suffix) {
        return false;
    }
    let mid = &channel[pattern.prefix.len()..channel.len() - pattern.suffix.len()];
    let Ok(n) = mid.parse::<i64>() else {
        return false;
    };
    if n < pattern.start {
        return false;
    }
    match pattern.end {
        Some(end) => n <= end,
        None => true,
    }
}

pub fn component_pattern_matches(pattern: &ComponentPattern<'_>, channel: &str) -> bool {
    if !channel.starts_with(pattern.prefix) || !channel.ends_with(pattern.suffix) {
        return false;
    }
    let mid = &channel[pattern.prefix.len()..channel.len() - pattern.suffix.len()];
    if mid.len() != 1 {
        return false;
    }
    pattern.components.chars().any(|c| mid == c.to_string())
}

pub fn pattern_matches_channel(pattern: &str, channel: &str) -> bool {
    pattern.split_whitespace().any(|pat| {
        if let Some(np) = parse_numbered_pattern(pat) {
            numbered_pattern_matches(&np, channel)
        } else if let Some(cp) = parse_component_pattern(pat) {
            component_pattern_matches(&cp, channel)
        } else {
            glob_match(pat, channel)
        }
    })
}

pub fn rename_to_at(template: &str, index: usize, keep_original: &str) -> String {
    let template = template.trim();
    if template.is_empty() {
        return keep_original.to_string();
    }
    if let Some(NamePattern::Components(p)) = parse_name_pattern(template) {
        if let Some(name) = component_name_at(&p, index) {
            return name;
        }
        return keep_original.to_string();
    }
    if let Some(pat) = parse_numbered_pattern(template) {
        return channel_name_at(&pat, index);
    }
    template
        .split_whitespace()
        .nth(index)
        .map(str::to_string)
        .unwrap_or_else(|| keep_original.to_string())
}

fn glob_match(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some(("", suffix)) => name.ends_with(suffix),
        Some((prefix, "")) => name.starts_with(prefix),
        Some((prefix, suffix)) => name.starts_with(prefix) && name.ends_with(suffix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_from_closed_range() {
        assert_eq!(channel_count_from_name("chan[1-4]"), 4);
        assert_eq!(channel_count_from_name("chan[1-16]"), 16);
        assert_eq!(channel_count_from_name("wave[10]"), 1);
        assert_eq!(channel_count_from_name("noise"), 1);
        assert_eq!(channel_count_from_name("chan[1-]"), 1);
        assert_eq!(channel_count_from_name("t[xyz]"), 3);
    }

    #[test]
    fn component_pattern_expands_translate_channels() {
        assert_eq!(
            expand_channel_names("t[xyz]", 3),
            vec!["tx", "ty", "tz"]
        );
        assert_eq!(
            expand_channel_names("r[xyz]", 3),
            vec!["rx", "ry", "rz"]
        );
        assert_eq!(expand_channel_names("s[xy]", 2), vec!["sx", "sy"]);
        assert_eq!(
            expand_channel_name_field("t[xyz]"),
            vec!["tx", "ty", "tz"]
        );
    }

    #[test]
    fn expand_field_whitespace_and_range() {
        assert_eq!(
            expand_channel_name_field("chan[1-3]"),
            vec!["chan1", "chan2", "chan3"]
        );
        assert_eq!(
            expand_channel_name_field("tx ty tz"),
            vec!["tx", "ty", "tz"]
        );
        assert_eq!(
            expand_channel_name_field("chan[1-2] tail"),
            vec!["chan1", "chan2", "tail"]
        );
        assert_eq!(
            expand_channel_name_field("t[xyz] bass"),
            vec!["tx", "ty", "tz", "bass"]
        );
    }

    #[test]
    fn expands_closed_range() {
        assert_eq!(
            expand_channel_names("chan[1-4]", 4),
            vec!["chan1", "chan2", "chan3", "chan4"]
        );
    }

    #[test]
    fn expands_past_range_end_for_extra_channels() {
        assert_eq!(
            expand_channel_names("chan[1-3]", 5),
            vec!["chan1", "chan2", "chan3", "chan4", "chan5"]
        );
    }

    #[test]
    fn select_pattern_matches_range() {
        assert!(pattern_matches_channel("chan[1-3]", "chan2"));
        assert!(!pattern_matches_channel("chan[1-3]", "chan4"));
        assert!(pattern_matches_channel("tx*", "tx1"));
        assert!(pattern_matches_channel("t[xyz]", "ty"));
        assert!(!pattern_matches_channel("t[xyz]", "t1"));
    }

    #[test]
    fn space_separates_channel_names() {
        assert_eq!(channel_count_from_name("chan1 chan2"), 2);
        assert_eq!(
            expand_channel_names("chan1 chan2", 2),
            vec!["chan1", "chan2"]
        );
        assert_eq!(
            expand_channel_names("chan[1-2] bass", 3),
            vec!["chan1", "chan2", "bass"]
        );
    }

    #[test]
    fn rename_to_at_numbered() {
        assert_eq!(rename_to_at("out[1-]", 0, "x"), "out1");
        assert_eq!(rename_to_at("out[1-]", 2, "x"), "out3");
        assert_eq!(rename_to_at("a b", 1, "x"), "b");
        assert_eq!(rename_to_at("", 0, "keep"), "keep");
        assert_eq!(rename_to_at("t[xyz]", 0, "x"), "tx");
        assert_eq!(rename_to_at("t[xyz]", 2, "x"), "tz");
    }
}
