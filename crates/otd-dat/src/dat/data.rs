use otd_core::ChopTableMirror;

use crate::chop_mirror;

/// A DAT's contents: rows of cells.
///
/// A "text" DAT is simply one cell; keeping one representation means Select,
/// Merge and the viewers do not need two code paths.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DatData {
    pub rows: Vec<Vec<String>>,
    /// Presented as free text rather than as a grid.
    pub is_text: bool,
    /// Large CHOP→DAT: grid is read from the channel store on demand.
    pub chop_mirror: Option<ChopTableMirror>,
}

impl DatData {
    pub fn text(s: impl Into<String>) -> Self {
        DatData {
            rows: vec![vec![s.into()]],
            is_text: true,
            chop_mirror: None,
        }
    }

    pub fn table(rows: Vec<Vec<String>>) -> Self {
        DatData {
            rows,
            is_text: false,
            chop_mirror: None,
        }
    }

    pub fn chop_mirror(mirror: ChopTableMirror) -> Self {
        DatData {
            rows: Vec::new(),
            is_text: false,
            chop_mirror: Some(mirror),
        }
    }

    pub fn num_rows(&self) -> usize {
        if let Some(m) = &self.chop_mirror {
            return chop_mirror::grid_size(m).0;
        }
        self.rows.len()
    }

    pub fn num_cols(&self) -> usize {
        if let Some(m) = &self.chop_mirror {
            return chop_mirror::grid_size(m).1;
        }
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    pub fn cell(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// The whole thing as text — a text DAT's single cell, or the table as
    /// tab-separated rows.
    pub fn as_text(&self) -> String {
        if self.is_text {
            return self.cell(0, 0).to_string();
        }
        self.rows
            .iter()
            .map(|r| r.join("\t"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse tab- or comma-separated text into a table.
    pub fn from_delimited(text: &str, delimiter: char) -> Self {
        let rows: Vec<Vec<String>> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.split(delimiter).map(|c| c.trim().to_string()).collect())
            .collect();
        DatData::table(rows)
    }

    /// Look a value up by its row heading, the way a cue table is used.
    pub fn lookup(&self, key: &str, column: usize) -> Option<&str> {
        self.rows
            .iter()
            .find(|r| r.first().map(|c| c == key).unwrap_or(false))
            .and_then(|r| r.get(column))
            .map(|s| s.as_str())
    }

    pub fn set_cell(&mut self, row: usize, col: usize, value: impl Into<String>) {
        while self.rows.len() <= row {
            self.rows.push(Vec::new());
        }
        while self.rows[row].len() <= col {
            self.rows[row].push(String::new());
        }
        self.rows[row][col] = value.into();
        if self.is_text && (row > 0 || col > 0 || self.rows.len() > 1) {
            self.is_text = false;
        }
    }

    pub fn clear_grid(&mut self) {
        self.rows.clear();
        if self.is_text {
            self.rows.push(vec![String::new()]);
        }
    }

    pub fn append_row(&mut self, cells: &[impl AsRef<str>]) {
        self.is_text = false;
        self.rows
            .push(cells.iter().map(|c| c.as_ref().to_string()).collect());
    }

    pub fn append_col(&mut self, default: &str) {
        self.is_text = false;
        let target = self.num_cols().max(1);
        if self.rows.is_empty() {
            self.rows.push(vec![default.to_string()]);
            return;
        }
        for row in &mut self.rows {
            while row.len() < target {
                row.push(String::new());
            }
            row.push(default.to_string());
        }
    }

    pub fn insert_row(&mut self, index: usize, cells: &[impl AsRef<str>]) {
        self.is_text = false;
        let row: Vec<String> = cells.iter().map(|c| c.as_ref().to_string()).collect();
        if index >= self.rows.len() {
            self.rows.push(row);
        } else {
            self.rows.insert(index, row);
        }
    }

    pub fn delete_row(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
        }
    }

    pub fn to_param_text(&self) -> String {
        self.as_text()
    }

    pub fn apply_cell_write(grid: &mut Vec<Vec<String>>, row: usize, col: usize, value: &str) {
        while grid.len() <= row {
            grid.push(Vec::new());
        }
        while grid[row].len() <= col {
            grid[row].push(String::new());
        }
        grid[row][col] = value.to_string();
    }
}
