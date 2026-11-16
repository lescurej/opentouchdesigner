use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::chop_mirror::format_number;
use crate::DatData;

use super::{DatCtx, DatSpec, spec as dat_spec};
pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "chop" => Param::str("").with_label("CHOP").as_path_ref(),
        "layout" => Param::menu("columns", &["columns", "rows"])
            .with_label("Channels As"),
        "names" => Param::bool(true).with_label("Include Names"),
        "format" => Param::str("%.6g").with_label("Number Format"),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let Some((names, samples, _)) = c.foreign(0).and_then(|f| f.as_channels()) else {
        return DatData::table(Vec::new());
    };
    let fmt = c.s("format");
    let show_names = c.b("names");
    let num = |v: f32| format_number(&fmt, v);

    let length = samples.iter().map(|s| s.len()).max().unwrap_or(0);
    let mut rows: Vec<Vec<String>> = Vec::new();

    if c.menu("layout") == 0 {
        // A column per channel, a row per sample — the spreadsheet shape.
        if show_names {
            rows.push(names.to_vec());
        }
        for i in 0..length {
            rows.push(
                samples
                    .iter()
                    .map(|s| s.get(i).copied().map(num).unwrap_or_default())
                    .collect(),
            );
        }
    } else {
        for (name, s) in names.iter().zip(samples) {
            let mut row = Vec::with_capacity(s.len() + 1);
            if show_names {
                row.push(name.clone());
            }
            row.extend(s.iter().copied().map(num));
            rows.push(row);
        }
    }
    DatData::table(rows)
}

pub const TYPE_NAME: &str = "choptodatDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "CHOP to DAT",
        &[],
        "Channels as table.",
        params,
        cook,
    )
}
