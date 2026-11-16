use otd_core::Value;

use crate::DatData;

use super::{DatCtx, constants::*};

pub(crate) fn resolve_dat_op(c: &DatCtx<'_>, path: &str) -> Option<otd_core::NodeId> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        c.graph.find(path)
    } else {
        c.graph.find_from(c.id, path)
    }
}

pub(crate) fn dat_script_path(c: &DatCtx<'_>) -> String {
    c.graph
        .get(c.id)
        .and_then(|n| n.parent)
        .map(|parent| c.graph.path(parent))
        .unwrap_or_else(|| c.path.to_string())
}

pub(crate) fn callback_dat_view(c: &DatCtx<'_>, source: &str) -> DatData {
    if !crate::callbacks::uses_external_callbacks_dat(c.graph, c.id) {
        return DatData::text(source.to_string());
    }
    match c.node.op_type.as_str() {
        EXECUTE => DatData::table(vec![
            vec!["name".into(), "value".into()],
            vec!["frame".into(), c.eval.frame.to_string()],
        ]),
        "chopexecuteDAT" | "parameterexecuteDAT" => {
            let mut rows = vec![vec!["name".into(), "value".into()]];
            let mut keys: Vec<_> = c.watched.values.keys().cloned().collect();
            keys.sort();
            for k in keys {
                let v = c.watched.values[&k];
                rows.push(vec![k, v.to_string()]);
            }
            DatData::table(rows)
        }
        PANEL_EXECUTE => {
            let v = c.watched.values.get("panel").copied().unwrap_or(0.0);
            DatData::table(vec![
                vec!["panel".into(), "value".into()],
                vec!["panel".into(), v.to_string()],
            ])
        }
        WEBSERVER => {
            let port = c.val("port").as_i64();
            let addr = c.s("localaddress");
            DatData::table(vec![
                vec!["name".into(), "value".into()],
                vec!["port".into(), port.to_string()],
                vec!["address".into(), addr],
            ])
        }
        _ => DatData::default(),
    }
}

/// Fire one callback, reporting a script error on the node.
///
/// A failing callback disables nothing and stops nothing: the same rule the
/// rest of the engine follows for a bad expression or a missing device, and
/// for the same reason — a typo during a show must not take the render with
/// it.
pub(crate) fn fire(c: &mut DatCtx, source: &str, func: &str, args: &[Value]) {
    let script = dat_script_path(c);
    fire_at(c, source, func, args, &script);
}

pub(crate) fn fire_dat_callback(c: &mut DatCtx, func: &str, args: &[Value]) {
    let source = c.callback_script();
    if source.trim().is_empty() {
        return;
    }
    fire(c, &source, func, args);
}

pub(crate) fn fire_new_table_rows(
    c: &mut DatCtx,
    watched_key: &str,
    table: &DatData,
    callback: &str,
    row_args: impl Fn(usize, &DatData) -> Vec<Value>,
) {
    let source = c.callback_script();
    if source.trim().is_empty() {
        return;
    }
    let data_rows = table.num_rows().saturating_sub(1);
    let prev = c.watched.values.get(watched_key).copied().unwrap_or(0.0) as usize;
    if data_rows < prev {
        c.watched
            .values
            .insert(watched_key.into(), data_rows as f64);
        return;
    }
    for row in (prev + 1)..=data_rows {
        fire(c, &source, callback, &row_args(row, table));
    }
    c.watched
        .values
        .insert(watched_key.into(), data_rows as f64);
}

pub(crate) fn ws_set_open(c: &mut DatCtx, open: bool) {
    let was = c.watched.values.get("_ws_open").copied().unwrap_or(0.0) > 0.5;
    if open && !was {
        fire_dat_callback(c, "onOpen", &[]);
        c.watched.values.insert("_ws_open".into(), 1.0);
    } else if !open && was {
        fire_dat_callback(c, "onClose", &[]);
        c.watched.values.insert("_ws_open".into(), 0.0);
    }
}

pub(crate) fn fire_at(c: &mut DatCtx, source: &str, func: &str, args: &[Value], script_path: &str) {
    let Some(scripts) = c.scripts else { return };
    if let Err(e) = scripts.call(source, func, args, c.eval, script_path) {
        if c.error.is_none() {
            c.error = Some(e);
        }
    }
}

