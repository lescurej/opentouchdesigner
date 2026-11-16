use otd_core::ChopTableMirror;

pub fn grid_size(m: &ChopTableMirror) -> (usize, usize) {
    if m.layout_columns {
        let rows = m.samples + usize::from(m.include_names);
        let cols = m.channels;
        (rows, cols)
    } else {
        let rows = m.channels;
        let cols = m.samples + usize::from(m.include_names);
        (rows, cols)
    }
}

pub fn format_number(spec: &str, v: f32) -> String {
    let digits = spec
        .trim_start_matches(['%', '.'])
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<usize>()
        .unwrap_or(6)
        .min(17);
    if spec.ends_with('f') {
        format!("{v:.digits$}")
    } else {
        let s = format!("{:.*}", digits, v);
        let s = if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        };
        if s == "-0" { "0".into() } else { s }
    }
}

pub fn cell_string_slices(
    m: &ChopTableMirror,
    names: &[&str],
    samples: &[&[f32]],
    row: usize,
    col: usize,
) -> String {
    if m.layout_columns {
        if m.include_names {
            if row == 0 {
                return names.get(col).unwrap_or(&"").to_string();
            }
            let si = row - 1;
            return samples
                .get(col)
                .and_then(|s| s.get(si))
                .copied()
                .map(|v| format_number(&m.format, v))
                .unwrap_or_default();
        }
        return samples
            .get(col)
            .and_then(|s| s.get(row))
            .copied()
            .map(|v| format_number(&m.format, v))
            .unwrap_or_default();
    }
    if m.include_names {
        if col == 0 {
            return names.get(row).unwrap_or(&"").to_string();
        }
        let si = col - 1;
        return samples
            .get(row)
            .and_then(|s| s.get(si))
            .copied()
            .map(|v| format_number(&m.format, v))
            .unwrap_or_default();
    }
    samples
        .get(row)
        .and_then(|s| s.get(col))
        .copied()
        .map(|v| format_number(&m.format, v))
        .unwrap_or_default()
}

pub fn cell_string(
    m: &ChopTableMirror,
    names: &[String],
    samples: &[Vec<f32>],
    row: usize,
    col: usize,
) -> String {
    if m.layout_columns {
        if m.include_names {
            if row == 0 {
                return names.get(col).cloned().unwrap_or_default();
            }
            let si = row - 1;
            return samples
                .get(col)
                .and_then(|s| s.get(si))
                .copied()
                .map(|v| format_number(&m.format, v))
                .unwrap_or_default();
        }
        return samples
            .get(col)
            .and_then(|s| s.get(row))
            .copied()
            .map(|v| format_number(&m.format, v))
            .unwrap_or_default();
    }
    if m.include_names {
        if col == 0 {
            return names.get(row).cloned().unwrap_or_default();
        }
        let si = col - 1;
        return samples
            .get(row)
            .and_then(|s| s.get(si))
            .copied()
            .map(|v| format_number(&m.format, v))
            .unwrap_or_default();
    }
    samples
        .get(row)
        .and_then(|s| s.get(col))
        .copied()
        .map(|v| format_number(&m.format, v))
        .unwrap_or_default()
}

pub fn materialize_table(
    m: &ChopTableMirror,
    names: &[String],
    samples: &[Vec<f32>],
) -> Vec<Vec<String>> {
    let (nrows, ncols) = grid_size(m);
    let mut rows = Vec::with_capacity(nrows);
    for r in 0..nrows {
        let mut row = Vec::with_capacity(ncols);
        for c in 0..ncols {
            row.push(cell_string(m, names, samples, r, c));
        }
        rows.push(row);
    }
    rows
}
