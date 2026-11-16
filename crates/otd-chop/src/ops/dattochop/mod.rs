use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "dat" => Param::str("").with_label("DAT").as_path_ref(),
        "layout" => Param::menu("columns", &["columns", "rows"])
            .with_label("Channels From"),
        "names" => Param::bool(true)
            .with_label("First Row/Column Is Names"),
    }
}

/// Numbers out of a table. A cue list, a CSV of positions, a JSON payload
/// already flattened by a JSON DAT — all of them become channels here, which
/// is the point of the DAT family being a family at all.
pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let Some(rows) = c.foreign(0).and_then(|f| f.as_table()) else {
        return ChopData::empty();
    };
    let by_columns = c.menu("layout") == 0;
    let named = c.b("names");

    // Work in "lines of cells", where a line is a column or a row depending
    // on the layout, so the two cases share everything below.
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let cell = |r: usize, col: usize| -> &str {
        rows.get(r)
            .and_then(|row| row.get(col))
            .map(|s| s.as_str())
            .unwrap_or("")
    };
    let lines: Vec<Vec<String>> = if by_columns {
        (0..width)
            .map(|col| (0..rows.len()).map(|r| cell(r, col).to_string()).collect())
            .collect()
    } else {
        rows.iter().map(|r| r.to_vec()).collect()
    };

    let channels: Vec<Channel> = lines
        .into_iter()
        .enumerate()
        .map(|(i, mut line)| {
            let name = if named && !line.is_empty() {
                let head = line.remove(0);
                if head.trim().is_empty() {
                    format!("chan{}", i + 1)
                } else {
                    crate::ops::shared::sanitise::sanitise(&head)
                }
            } else {
                format!("chan{}", i + 1)
            };
            // A cell that is not a number is 0 rather than an error: a table
            // usually has a label column, and refusing to cook because of it
            // would make the common case the broken one.
            let samples = line
                .iter()
                .map(|s| s.trim().parse().unwrap_or(0.0))
                .collect();
            Channel::new(name, samples)
        })
        .filter(|c| !c.samples.is_empty())
        .collect();

    // Not time sliced: this is a standalone buffer, like a Pattern CHOP.
    ChopData::new(channels, CONTROL_RATE, false)
}



pub const TYPE_NAME: &str = "dattochopCHOP";

pub fn spec() -> ChopSpec {
    super::converter_spec(
                TYPE_NAME,
                "DAT to CHOP",
                "Columns or rows of a table, as channels.",
                params,
                cook,
            )
}
