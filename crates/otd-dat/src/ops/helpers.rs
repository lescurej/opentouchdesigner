use otd_core::indexmap::IndexMap;
use otd_core::Param;

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
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

pub fn glob(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((head, tail)) => {
            name.len() >= head.len() + tail.len() && name.starts_with(head) && name.ends_with(tail)
        }
    }
}
