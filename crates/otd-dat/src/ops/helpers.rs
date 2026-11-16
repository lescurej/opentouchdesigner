use otd_core::Param;
use otd_core::indexmap::IndexMap;

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

pub(crate) fn params_missing() -> IndexMap<String, Param> {
    otd_core::missing::missing_params()
}

pub(crate) fn no_params() -> IndexMap<String, Param> {
    params! {}
}

pub(crate) fn selection(spec: &str, headings: &[String], count: usize) -> Vec<usize> {
    let spec = spec.trim();
    if spec.is_empty() || spec == "*" {
        return (0..count).collect();
    }
    let mut out = Vec::new();
    for token in spec.split_whitespace() {
        if let Ok(i) = token.parse::<usize>() {
            if i < count && !out.contains(&i) {
                out.push(i);
            }
            continue;
        }
        for (i, h) in headings.iter().enumerate() {
            if glob(token, h) && !out.contains(&i) {
                out.push(i);
            }
        }
    }
    out
}

pub(crate) fn selected<'a>(pattern: &str, names: &'a [String]) -> Vec<&'a String> {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern == "*" {
        return names.iter().collect();
    }
    let wanted: Vec<&str> = pattern.split_whitespace().collect();
    names
        .iter()
        .filter(|n| {
            wanted.iter().any(|w| {
                w.strip_suffix('*')
                    .map(|prefix| n.starts_with(prefix))
                    .unwrap_or(*w == n.as_str())
            })
        })
        .collect()
}

pub fn glob(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((head, tail)) => {
            name.len() >= head.len() + tail.len() && name.starts_with(head) && name.ends_with(tail)
        }
    }
}

pub(crate) fn network_callback_params(default_source: &'static str) -> IndexMap<String, Param> {
    params! {
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(default_source).with_label("Source").into_script(),
    }
}

pub(crate) fn merge_params(
    mut base: IndexMap<String, Param>,
    callbacks: IndexMap<String, Param>,
) -> IndexMap<String, Param> {
    base.extend(callbacks);
    base
}
