use quick_xml::events::Event;
use quick_xml::Reader;

use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "root" => Param::str("").with_label("Element Path Filter"),
    }
}

fn flatten_xml_text(text: &str, out: &mut Vec<Vec<String>>) {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    while let Ok(event) = reader.read_event_into(&mut buf) {
        match event {
            Event::Start(e) => {
                let name = String::from_utf8_lossy(e.local_name().as_ref()).into_owned();
                let path = if stack.is_empty() {
                    format!("/{name}")
                } else {
                    format!("{}/{name}", stack.last().unwrap())
                };
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref());
                    let val = String::from_utf8_lossy(&attr.value);
                    out.push(vec![format!("{path}@{key}"), val.into_owned()]);
                }
                stack.push(path);
            }
            Event::Text(t) => {
                let text = t.unescape().unwrap_or_default().trim().to_string();
                if !text.is_empty() {
                    if let Some(path) = stack.last() {
                        out.push(vec![path.clone(), text]);
                    }
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let text = c.input(0).as_text();
    if text.trim().is_empty() {
        return DatData::table(vec![vec!["path".into(), "value".into()]]);
    }
    let mut rows = vec![vec!["path".into(), "value".into()]];
    flatten_xml_text(&text, &mut rows);
    let filter = c.s("root").trim().to_string();
    if filter.is_empty() {
        return DatData::table(rows);
    }
    let filtered: Vec<Vec<String>> = rows
        .into_iter()
        .filter(|r| r.first().map(|p| p.starts_with(&filter)).unwrap_or(false))
        .collect();
    DatData::table(filtered)
}
pub const TYPE_NAME: &str = "xmlDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "XML",
        &["in"],
        "Parse XML to table.",
        params,
        cook,
    )
}
