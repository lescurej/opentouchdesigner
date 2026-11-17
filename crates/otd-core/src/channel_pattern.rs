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
        return (0..count).map(|i| channel_name_at(&pat, i)).collect();
    }
    if count == 1 {
        vec![token.to_string()]
    } else {
        (0..count).map(|i| format!("{token}{}", i + 1)).collect()
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

fn pattern_token_matches(pat: &str, channel: &str) -> bool {
    if let Some(np) = parse_numbered_pattern(pat) {
        numbered_pattern_matches(&np, channel)
    } else if let Some(cp) = parse_component_pattern(pat) {
        component_pattern_matches(&cp, channel)
    } else {
        glob_match(pat, channel)
    }
}

fn is_exclude_token(pat: &str) -> bool {
    pat.len() > 1 && pat.starts_with('^')
}

fn exclusion_only(pattern: &str) -> bool {
    let mut any = false;
    for pat in pattern.split_whitespace() {
        any = true;
        if !is_exclude_token(pat) {
            return false;
        }
    }
    any
}

pub fn pattern_matches_channel(pattern: &str, channel: &str) -> bool {
    let mut includes = Vec::new();
    let mut excludes = Vec::new();
    for pat in pattern.split_whitespace() {
        if let Some(exclude) = pat.strip_prefix('^') {
            if !exclude.is_empty() {
                excludes.push(exclude);
            }
        } else {
            includes.push(pat);
        }
    }
    if excludes
        .iter()
        .any(|pat| pattern_token_matches(pat, channel))
    {
        return false;
    }
    if includes.is_empty() {
        return !excludes.is_empty();
    }
    includes
        .iter()
        .any(|pat| pattern_token_matches(pat, channel))
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
    let pat: Vec<char> = pattern.chars().collect();
    let name: Vec<char> = name.chars().collect();
    let mut pi = 0;
    let mut ni = 0;
    let mut star_p: Option<usize> = None;
    let mut star_n = 0;
    while ni < name.len() {
        if pi < pat.len() && (pat[pi] == '?' || pat[pi] == name[ni]) {
            pi += 1;
            ni += 1;
        } else if pi < pat.len() && pat[pi] == '*' {
            star_p = Some(pi);
            star_n = ni;
            pi += 1;
        } else if let Some(sp) = star_p {
            star_n += 1;
            ni = star_n;
            pi = sp + 1;
        } else {
            return false;
        }
    }
    while pi < pat.len() && pat[pi] == '*' {
        pi += 1;
    }
    pi == pat.len()
}

pub fn operator_reference_is_pattern(reference: &str) -> bool {
    let reference = reference.trim();
    reference.contains('*')
        || reference.contains('?')
        || reference.contains('[')
        || reference.contains('^')
}

pub fn pattern_matches_operator(pattern: &str, name: &str, path: &str) -> bool {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern == "*" {
        return false;
    }
    let exclude_only = exclusion_only(pattern);
    if !exclude_only && pattern_matches_channel(pattern, name) {
        return true;
    }
    let path_rel = path.trim_start_matches('/');
    if glob_match(pattern, name) || glob_match(pattern, path_rel) {
        return true;
    }
    if let Some(tail) = pattern.strip_prefix("*/") {
        if tail.is_empty() {
            return false;
        }
        if name == tail
            || pattern_matches_channel(tail, name)
            || path.ends_with(&format!("/{tail}"))
            || glob_match(&format!("*{tail}"), name)
        {
            return true;
        }
    }
    !exclude_only && pattern_matches_channel(pattern, path_rel)
}

#[cfg(test)]
#[path = "tests/channel_pattern__tests.rs"]
mod tests;
