use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::DatData;

use super::callbacks::resolve_dat_op;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "component" => Param::str("..").with_label("Panel COMP").as_path_ref(),
        "mode" => Param::menu("list", &["list", "table"]).with_label("Mode"),
        "visible" => Param::int(24).with_label("Visible Rows").with_range(1.0, 128.0),
    }
}

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
    let Some(comp) = resolve_dat_op(c, &c.s("component")) else {
        return DatData::text(String::new());
    };
    let comp_node = c.graph.node(comp);
    let table_path = comp_node
        .param("table")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    let Some(table_id) = c.graph.find_from(comp, table_path.trim()) else {
        return DatData::text("Set Table parameter");
    };
    let table_node = c.graph.node(table_id);
    let raw = table_node
        .param("text")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    let data = DatData::from_delimited(&raw, '\t');
    if data.num_rows() == 0 {
        return DatData::text("(empty)");
    }
    let visible = c.val("visible").as_f64().round().clamp(1.0, 128.0) as usize;
    if c.menu("mode") == 1 {
        let scroll_x = comp_node
            .param("scrollx")
            .map(|p| p.eval(c.eval).as_f64())
            .unwrap_or(0.0) as f32;
        let scroll_y = comp_node
            .param("scrolly")
            .map(|p| p.eval(c.eval).as_f64())
            .unwrap_or(0.0) as f32;
        let cols = data.num_cols().max(1);
        let _col_w = 12usize;
        let start_col = (scroll_x * cols as f32).floor() as usize;
        let start_row = (scroll_y * data.num_rows() as f32).floor() as usize;
        let mut lines = Vec::new();
        for r in start_row..(start_row + visible).min(data.num_rows()) {
            let mut line = String::new();
            for cidx in start_col..(start_col + 8).min(cols) {
                if !line.is_empty() {
                    line.push('\t');
                }
                line.push_str(data.cell(r, cidx));
            }
            lines.push(line);
        }
        return DatData::text(lines.join("\n"));
    }
    let scroll = comp_node
        .param("scroll")
        .map(|p| p.eval(c.eval).as_f64())
        .unwrap_or(0.0) as f32;
    let row_h = comp_node
        .param("rowheight")
        .map(|p| p.eval(c.eval).as_f64())
        .unwrap_or(0.08) as f32;
    let start = (scroll / row_h.max(0.001)).floor() as usize;
    let selected = comp_node
        .param("value")
        .map(|p| p.eval(c.eval).as_f64())
        .unwrap_or(0.0)
        .round() as i32;
    let mut lines = Vec::new();
    for (i, row) in (start..data.num_rows()).take(visible).enumerate() {
        let idx = (start + i) as i32;
        let mark = if idx == selected { "> " } else { "  " };
        let cell = if data.num_cols() > 0 {
            data.cell(row, 0)
        } else {
            ""
        };
        lines.push(format!("{mark}{cell}"));
    }
    DatData::text(lines.join("\n"))
}

pub const TYPE_NAME: &str = "paneltabletextDAT";

pub fn spec() -> DatSpec {
    dat_spec(
        TYPE_NAME,
        "Panel Table Text",
        &[],
        "Panel list/table as text.",
        params,
        cook,
    )
}
