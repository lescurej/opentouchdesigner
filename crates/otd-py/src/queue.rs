use super::*;

pub(crate) fn after_script_invoke() -> Result<(), String> {
    flush_pending_sync_recooks()
}

pub(crate) fn queue_dat_text(path: &str, text: String) {
    let param = with_network(|net, _| net.dat_content_param(path))
        .flatten()
        .unwrap_or("text");
    script_actions().queue_dat_text(path, param, text);
}

pub(crate) fn queue_recook(path: &str, deferred: bool) {
    script_actions().run_op(path, deferred);
}

pub(crate) fn dat_grid_mut(path: &str, edit: impl FnOnce(&mut DatData) -> Result<(), String>) -> Result<(), String> {
    let mut data = DatData::table(dat_grid(path).unwrap_or_default());
    edit(&mut data)?;
    queue_dat_text(path, data.to_param_text());
    Ok(())
}

pub(crate) fn dat_grid(path: &str) -> Option<Vec<Vec<String>>> {
    with_network(|net, _| net.dat_rows(path)).flatten()
}

pub(crate) fn dat_num_rows(path: &str) -> i64 {
    dat_grid(path).map(|g| g.len() as i64).unwrap_or(0)
}

pub(crate) fn dat_num_cols(path: &str) -> i64 {
    dat_grid(path)
        .map(|g| g.iter().map(|r| r.len()).max().unwrap_or(0) as i64)
        .unwrap_or(0)
}

pub(crate) fn dat_get_cell(path: &str, row: i64, col: i64) -> String {
    if row < 0 || col < 0 {
        return String::new();
    }
    let row = row as usize;
    let col = col as usize;
    dat_grid(path)
        .and_then(|g| g.get(row).and_then(|r| r.get(col)).cloned())
        .unwrap_or_default()
}

pub(crate) fn dat_set_cell(path: &str, row: i64, col: i64, value: &str) -> Result<(), String> {
    if row < 0 || col < 0 {
        return Err("row and column must be >= 0".into());
    }
    dat_grid_mut(path, |data| {
        data.set_cell(row as usize, col as usize, value);
        Ok(())
    })
}

pub(crate) fn dat_clear(path: &str) {
    let _ = dat_grid_mut(path, |data| {
        data.clear_grid();
        Ok(())
    });
}
