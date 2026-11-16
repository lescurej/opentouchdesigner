use crate::DatData;

const CAP: usize = 500;

#[derive(Clone, Debug, Default)]
pub struct MonitorLog {
    rows: Vec<Vec<String>>,
}

impl MonitorLog {
    pub fn push(&mut self, frame: i64, source: &str, kind: &str, detail: &str) {
        if self.rows.is_empty() {
            self.rows.push(vec![
                "frame".into(),
                "source".into(),
                "kind".into(),
                "detail".into(),
            ]);
        }
        self.rows.push(vec![
            frame.to_string(),
            source.to_string(),
            kind.to_string(),
            detail.to_string(),
        ]);
        while self.rows.len() > CAP + 1 {
            self.rows.remove(1);
        }
    }

    pub fn table(&self) -> DatData {
        if self.rows.is_empty() {
            return DatData::table(vec![vec![
                "frame".into(),
                "source".into(),
                "kind".into(),
                "detail".into(),
            ]]);
        }
        DatData::table(self.rows.clone())
    }
}
