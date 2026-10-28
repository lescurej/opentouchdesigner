#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumberedPattern<'a> {
    pub prefix: &'a str,
    pub start: i64,
    pub end: Option<i64>,
    pub suffix: &'a str,
}

pub fn parse_numbered_pattern(template: &str) -> Option<NumberedPattern<'_>> {
    let open = template.find('[')?;
    let close = template[open + 1..].find(']')? + open + 1;
    if close <= open + 1 {
        return None;
    }
    let prefix = &template[..open];
    let inner = template[open + 1..close].trim();
    let suffix = &template[close + 1..];
    let (start, end) = parse_range_inner(inner);
    Some(NumberedPattern {
        prefix,
        start,
        end,
        suffix,
    })
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

pub fn channel_name_at(pattern: &NumberedPattern<'_>, index: usize) -> String {
    let n = pattern.start.saturating_add(index as i64);
    format!("{}{}{}", pattern.prefix, n, pattern.suffix)
}

pub fn channel_count_from_name(template: &str) -> usize {
    let Some(pat) = parse_numbered_pattern(template.trim()) else {
        return 1;
    };
    match pat.end {
        Some(end) => ((end - pat.start + 1).max(1)) as usize,
        None => 1,
    }
}

pub fn expand_channel_names(template: &str, count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    if let Some(pat) = parse_numbered_pattern(template) {
        (0..count)
            .map(|i| channel_name_at(&pat, i))
            .collect()
    } else if count == 1 {
        vec![template.to_string()]
    } else {
        (0..count)
            .map(|i| format!("{template}{}", i + 1))
            .collect()
    }
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

pub fn pattern_matches_channel(pattern: &str, channel: &str) -> bool {
    pattern.split_whitespace().any(|pat| {
        if let Some(np) = parse_numbered_pattern(pat) {
            numbered_pattern_matches(&np, channel)
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
    }

    #[test]
    fn rename_to_at_numbered() {
        assert_eq!(rename_to_at("out[1-]", 0, "x"), "out1");
        assert_eq!(rename_to_at("out[1-]", 2, "x"), "out3");
        assert_eq!(rename_to_at("a b", 1, "x"), "b");
        assert_eq!(rename_to_at("", 0, "keep"), "keep");
    }
}
