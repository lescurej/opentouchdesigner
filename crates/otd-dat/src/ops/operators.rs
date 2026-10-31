use std::net::SocketAddr;

use otd_core::indexmap::IndexMap;
use otd_core::{EvalContext, Param, Value};

use crate::DatData;

use super::constants::*;
use super::helpers::*;
use super::DatCtx;

// ----------------------------------------------------------------- text

pub(crate) fn params_text() -> IndexMap<String, Param> {
    params! {
        "text" => Param::str("").with_label("Text"),
    }
}

pub(crate) fn cook_text(c: &mut DatCtx) -> DatData {
    DatData::text(c.s("text"))
}

// ---------------------------------------------------------------- table

pub(crate) fn params_table() -> IndexMap<String, Param> {
    params! {
        // The table's contents live in the project file, so a cue list is
        // versioned with the patch that uses it.
        "text" => Param::str("name\tvalue\n").with_label("Contents"),
        "delimiter" => Param::menu("tab", &["tab", "comma"]).with_label("Delimiter"),
    }
}

fn delimiter_of(c: &DatCtx) -> char {
    if c.menu("delimiter") == 1 { ',' } else { '\t' }
}

pub(crate) fn cook_table(c: &mut DatCtx) -> DatData {
    DatData::from_delimited(&c.s("text"), delimiter_of(c))
}

// --------------------------------------------------------------- select

pub(crate) fn params_select() -> IndexMap<String, Param> {
    params! {
        "rows" => Param::str("*").with_label("Rows (names, indices or *)"),
        "cols" => Param::str("*").with_label("Columns"),
        "byname" => Param::bool(true).with_label("First Row/Column Are Names"),
    }
}

pub(crate) fn cook_select(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let by_name = c.b("byname");

    let col_headings: Vec<String> = if by_name {
        input.rows.first().cloned().unwrap_or_default()
    } else {
        Vec::new()
    };
    let row_headings: Vec<String> = if by_name {
        input
            .rows
            .iter()
            .map(|r| r.first().cloned().unwrap_or_default())
            .collect()
    } else {
        Vec::new()
    };

    let rows = selection(&c.s("rows"), &row_headings, input.num_rows());
    let cols = selection(&c.s("cols"), &col_headings, input.num_cols());

    let picked = rows
        .iter()
        .map(|r| {
            cols.iter()
                .map(|col| input.cell(*r, *col).to_string())
                .collect()
        })
        .collect();
    DatData::table(picked)
}

// ---------------------------------------------------------------- merge

pub(crate) fn params_merge() -> IndexMap<String, Param> {
    params! {
        "how" => Param::menu("rows", &["rows", "columns"]).with_label("Merge By"),
    }
}

pub(crate) fn cook_merge(c: &mut DatCtx) -> DatData {
    let (a, b) = (c.input(0), c.input(1));
    if c.menu("how") == 1 {
        // Side by side, padding the shorter one.
        let n = a.num_rows().max(b.num_rows());
        let rows = (0..n)
            .map(|r| {
                let mut row: Vec<String> = a.rows.get(r).cloned().unwrap_or_default();
                row.resize(a.num_cols(), String::new());
                row.extend(b.rows.get(r).cloned().unwrap_or_default());
                row
            })
            .collect();
        DatData::table(rows)
    } else {
        let mut rows = a.rows.clone();
        rows.extend(b.rows.iter().cloned());
        DatData::table(rows)
    }
}

// ----------------------------------------------------------------- JSON

pub(crate) fn params_json() -> IndexMap<String, Param> {
    params! {
        "pointer" => Param::str("").with_label("JSON Pointer (e.g. /items/0)"),
    }
}

/// Flatten JSON into `path`/`value` rows.
///
/// A table is what the rest of the network can use; keeping the flattened
/// path as the key means a Select DAT can pull one field out by name.
fn flatten(prefix: &str, v: &serde_json::Value, out: &mut Vec<Vec<String>>) {
    match v {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                flatten(&format!("{prefix}/{k}"), v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                flatten(&format!("{prefix}/{i}"), v, out);
            }
        }
        serde_json::Value::Null => out.push(vec![prefix.to_string(), String::new()]),
        serde_json::Value::Bool(b) => out.push(vec![prefix.to_string(), b.to_string()]),
        serde_json::Value::Number(n) => out.push(vec![prefix.to_string(), n.to_string()]),
        serde_json::Value::String(s) => out.push(vec![prefix.to_string(), s.clone()]),
    }
}

pub(crate) fn cook_json(c: &mut DatCtx) -> DatData {
    let text = c.input(0).as_text();
    if text.trim().is_empty() {
        return DatData::table(vec![vec!["path".into(), "value".into()]]);
    }
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            c.error = Some(format!("JSON: {e}"));
            return DatData::table(vec![vec!["path".into(), "value".into()]]);
        }
    };
    let pointer = c.s("pointer");
    let root = if pointer.trim().is_empty() {
        &parsed
    } else {
        match parsed.pointer(pointer.trim()) {
            Some(v) => v,
            None => {
                c.error = Some(format!("no JSON at `{}`", pointer.trim()));
                return DatData::table(vec![vec!["path".into(), "value".into()]]);
            }
        }
    };
    let mut rows = vec![vec!["path".to_string(), "value".to_string()]];
    flatten("", root, &mut rows);
    DatData::table(rows)
}

// --------------------------------------------------------------- script

pub(crate) fn params_script() -> IndexMap<String, Param> {
    params! {
        "source" => Param::str(
            "# Leave a list of lists in `rows`.\nrows = [['frame'], [frame]]\n",
        ).with_label("Source").as_script(),
    }
}

pub(crate) fn cook_script(c: &mut DatCtx) -> DatData {
    let source = c.s("source");
    let Some(host) = c.scripts else {
        c.error = Some("no Python interpreter in this build".into());
        return DatData::default();
    };
    match host.run_table(&source, c.eval, c.path) {
        Ok(rows) => DatData::table(rows),
        Err(e) => {
            c.error = Some(e);
            DatData::default()
        }
    }
}

// ------------------------------------------------------------------- UDP

pub(crate) fn params_udp_common() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "localaddress" => Param::str("").with_label("Local Address"),
        "format" => Param::menu("permessage", &["permessage", "perline", "perbyte"]).with_label("Row Format"),
        "maxlines" => Param::int(20).with_label("Maximum Lines").with_range(1.0, 1000.0),
        "bytes" => Param::bool(false).with_label("Bytes Column"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(UDP_RECEIVE_CALLBACKS).with_label("Source").into_script(),
    }
}

pub(crate) fn params_udp_in() -> IndexMap<String, Param> {
    let mut p = params_udp_common();
    p.insert("port".into(), Param::int(7000).with_label("Port").with_range(1.0, 65535.0));
    p
}

fn udp_listener_key(port: u16, local: &str) -> String {
    format!("{local}\0{port}")
}

fn fire_udp_new_rows(c: &mut DatCtx, table: &DatData, bytes_col: bool) {
    let source = c.callback_script();
    if source.trim().is_empty() {
        return;
    }
    let data_rows = table.num_rows().saturating_sub(1);
    let prev = c
        .watched
        .values
        .get("_udp_rows")
        .copied()
        .unwrap_or(0.0) as usize;
    if data_rows < prev {
        c.watched.values.insert("_udp_rows".into(), data_rows as f64);
        return;
    }
    for row in (prev + 1)..=data_rows {
        let row_index = row;
        let message = table.cell(row, 2).to_string();
        let byte_str = if bytes_col {
            table.cell(row, 3).to_string()
        } else {
            String::new()
        };
        let peer = format!("{}:{}", table.cell(row, 0), table.cell(row, 1));
        fire(
            c,
            &source,
            "onReceive",
            &[
                Value::Int(row_index as i64),
                Value::Str(message),
                Value::Str(byte_str),
                Value::Str(peer),
            ],
        );
    }
    c.watched
        .values
        .insert("_udp_rows".into(), data_rows as f64);
}

pub(crate) fn cook_udp_in(c: &mut DatCtx) -> DatData {
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let path = c.path.to_string();
    let bytes_col = c.b("bytes");
    let active = c.b("active");

    if !active {
        c.net.udp_in.remove(&path);
        c.watched.values.insert("_udp_rows".into(), 0.0);
        return crate::udp::empty_table(bytes_col);
    }

    let key = udp_listener_key(port, &local);
    let needs_open = c
        .net
        .udp_in
        .get(&path)
        .map(|u| udp_listener_key(u.port, &u.local_address) != key)
        .unwrap_or(true);
    if needs_open {
        c.net.udp_in.remove(&path);
        match crate::udp::UdpListener::bind(port, &local) {
            Ok(u) => {
                c.watched.values.insert("_udp_rows".into(), 0.0);
                c.net.udp_in.insert(path.clone(), u);
            }
            Err(e) => {
                c.error = Some(e);
                return crate::udp::empty_table(bytes_col);
            }
        }
    }

    let Some(listener) = c.net.udp_in.get(&path) else {
        return crate::udp::empty_table(bytes_col);
    };
    let maxlines = crate::udp::maxlines_from_node(c.node);
    let format = c.menu("format");
    let inbox = listener
        .inbox
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();
    let table = crate::udp::build_table(&inbox, maxlines, format, bytes_col);
    fire_udp_new_rows(c, &table, bytes_col);
    table
}

pub(crate) fn params_udp_out() -> IndexMap<String, Param> {
    let mut p = params_udp_common();
    p.insert(
        "address".into(),
        Param::str("127.0.0.1").with_label("Network Address"),
    );
    p.insert(
        "port".into(),
        Param::int(7001).with_label("Port").with_range(1.0, 65535.0),
    );
    p.insert(
        "sendonchange".into(),
        Param::bool(false).with_label("Send Input On Change"),
    );
    p
}

pub(crate) fn cook_udp_out(c: &mut DatCtx) -> DatData {
    let host = c.s("address");
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let local = c.s("localaddress");
    let active = c.b("active");
    let bytes_col = c.b("bytes");
    let path = c.path.to_string();

    if !active {
        c.net.udp_out.remove(&path);
        c.watched.values.insert("_udp_rows".into(), 0.0);
        return crate::udp::empty_table(bytes_col);
    }

    let Ok(target) = format!("{}:{}", host.trim(), port).parse::<SocketAddr>() else {
        c.error = Some(format!("bad UDP address `{host}:{port}`"));
        return crate::udp::empty_table(bytes_col);
    };

    let target_key = format!("{target}\0{local}");
    let needs_open = c
        .net
        .udp_out
        .get(&path)
        .map(|u| format!("{}\0{}", u.target, u.listener.local_address) != target_key)
        .unwrap_or(true);
    if needs_open {
        match crate::udp::UdpOutConnection::open(target, &local) {
            Ok(u) => {
                c.watched.values.insert("_udp_rows".into(), 0.0);
                c.net.udp_out.insert(path.clone(), u);
            }
            Err(e) => {
                c.error = Some(e);
                return crate::udp::empty_table(bytes_col);
            }
        }
    }

    if c.b("sendonchange") {
        let text = c.input(0).as_text();
        if let Some(out) = c.net.udp_out.get_mut(&path) {
            if out.last_sent.as_deref() != Some(text.as_str()) && !text.is_empty() {
                if let Err(e) = out.listener.send_to(text.as_bytes(), out.target) {
                    c.error = Some(e);
                } else {
                    out.last_sent = Some(text);
                }
            }
        }
    }

    let Some(out) = c.net.udp_out.get(&path) else {
        return crate::udp::empty_table(bytes_col);
    };
    let maxlines = crate::udp::maxlines_from_node(c.node);
    let format = c.menu("format");
    let inbox = out
        .listener
        .inbox
        .lock()
        .map(|v| v.clone())
        .unwrap_or_default();
    let table = crate::udp::build_table(&inbox, maxlines, format, bytes_col);
    fire_udp_new_rows(c, &table, bytes_col);
    table
}

// ------------------------------------------------------------ pass-through

pub(crate) fn cook_null(c: &mut DatCtx) -> DatData {
    c.input(0)
}
// -------------------------------------------------------------- execute

pub(crate) fn params_execute() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn params_chop_execute() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "chop" => Param::str("").with_label("Watch CHOP").as_path_ref(),
        "channels" => Param::str("*").with_label("Channels"),
        "threshold" => Param::float(0.5).with_label("On Threshold").with_range(-10.0, 10.0),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(CHOP_EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn params_par_execute() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "op" => Param::str("").with_label("OPs").as_path_ref(),
        "parameters" => Param::str("*").with_label("Parameters"),
        "valuechange" => Param::bool(true).with_label("Value Change"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(PAR_EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

pub(crate) fn params_panel_execute() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "panels" => Param::str("..").with_label("Panels").as_path_ref(),
        "panelvalue" => Param::str("value").with_label("Panel Value"),
        "offtoon" => Param::bool(true).with_label("Off to On"),
        "ontooff" => Param::bool(true).with_label("On to Off"),
        "valuechange" => Param::bool(true).with_label("Value Change"),
        "whileon" => Param::bool(false).with_label("While On"),
        "whileoff" => Param::bool(false).with_label("While Off"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(PANEL_EXECUTE_SOURCE).with_label("Source").into_script(),
    }
}

fn resolve_dat_op(c: &DatCtx<'_>, path: &str) -> Option<otd_core::NodeId> {
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

fn dat_script_path(c: &DatCtx<'_>) -> String {
    c.graph
        .get(c.id)
        .and_then(|n| n.parent)
        .map(|parent| c.graph.path(parent))
        .unwrap_or_else(|| c.path.to_string())
}

fn callback_dat_view(c: &DatCtx<'_>, source: &str) -> DatData {
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

pub(crate) fn cook_panel_execute(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    let Some(panel_id) = resolve_dat_op(c, &c.s("panels")) else {
        return callback_dat_view(c, &source);
    };
    let panel_path = c.graph.path(panel_id);
    let key = c.s("panelvalue");
    let value = c
        .graph
        .get(panel_id)
        .and_then(|n| n.param(&key))
        .map(|p| {
            p.eval(&EvalContext {
                channels: c.eval.channels,
                path: Some(&panel_path),
                ..*c.eval
            })
            .as_f64()
        })
        .unwrap_or(0.0);
    let prev = c.watched.values.get("panel").copied();
    let on = value > 0.0;
    let script = dat_script_path(c);
    let pv = Value::Str(key.clone());

    if let Some(prev_val) = prev {
        let was_on = prev_val > 0.0;
        if c.b("offtoon") && !was_on && on {
            fire_at(c, &source, "onOffToOn", &[pv.clone()], &script);
        } else if c.b("ontooff") && was_on && !on {
            fire_at(c, &source, "onOnToOff", &[pv.clone()], &script);
        } else if c.b("valuechange") && (prev_val - value).abs() > f64::EPSILON {
            fire_at(
                c,
                &source,
                "onValueChange",
                &[pv.clone(), Value::Float(value), Value::Float(prev_val)],
                &script,
            );
        }
    }

    if prev != Some(value) {
        c.watched.values.insert("panel".into(), value);
    }

    if c.b("whileon") && on {
        fire_at(c, &source, "whileOn", &[Value::Str(key.clone())], &script);
    }
    if c.b("whileoff") && !on {
        fire_at(c, &source, "whileOff", &[Value::Str(key)], &script);
    }
    callback_dat_view(c, &source)
}

/// Fire one callback, reporting a script error on the node.
///
/// A failing callback disables nothing and stops nothing: the same rule the
/// rest of the engine follows for a bad expression or a missing device, and
/// for the same reason — a typo during a show must not take the render with
/// it.
fn fire(c: &mut DatCtx, source: &str, func: &str, args: &[Value]) {
    let script = dat_script_path(c);
    fire_at(c, source, func, args, &script);
}

fn fire_at(c: &mut DatCtx, source: &str, func: &str, args: &[Value], script_path: &str) {
    let Some(scripts) = c.scripts else { return };
    if let Err(e) = scripts.call(source, func, args, c.eval, script_path) {
        if c.error.is_none() {
            c.error = Some(e);
        }
    }
}

/// Execute DAT output is status (frame, watched values), not callback source.
/// Script lives in the linked Callbacks DAT when one is set.
pub(crate) fn cook_execute(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    if !c.watched.started {
        c.watched.started = true;
        fire(c, &source, "onStart", &[]);
    }
    let frame = Value::Int(c.eval.frame);
    fire(c, &source, "onFrameStart", std::slice::from_ref(&frame));
    // Both edges of the frame fire here, one cook apart in the same cook.
    // A genuine end-of-frame hook would have to run after every other node,
    // and the cook is demand-driven — there is no such moment to hang it on
    // that is not simply "later", so saying so beats pretending.
    fire(c, &source, "onFrameEnd", std::slice::from_ref(&frame));
    callback_dat_view(c, &source)
}

/// Which of `names` a whitespace-separated pattern selects.
fn selected<'a>(pattern: &str, names: &'a [String]) -> Vec<&'a String> {
    let pattern = pattern.trim();
    if pattern.is_empty() || pattern == "*" {
        return names.iter().collect();
    }
    let wanted: Vec<&str> = pattern.split_whitespace().collect();
    names
        .iter()
        .filter(|n| {
            wanted.iter().any(|w| {
                w.strip_suffix('*')
                    .map(|prefix| n.starts_with(prefix))
                    .unwrap_or(*w == n.as_str())
            })
        })
        .collect()
}

pub(crate) fn cook_chop_execute(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    let path = c.s("chop");
    let Some(net) = c.eval.channels else {
        return callback_dat_view(c, &source);
    };
    let names = net.channel_names(path.trim());
    let threshold = c
        .node
        .param("threshold")
        .map(|p| p.eval(c.eval).as_f32())
        .unwrap_or(0.5);
    let pattern = c.s("channels");

    // Collected first, because firing a callback borrows `c` mutably and the
    // network borrow has to be finished by then.
    let mut events: Vec<(String, f64, Option<f64>)> = Vec::new();
    for name in selected(&pattern, &names) {
        let Some(value) = net.channel(path.trim(), name) else {
            continue;
        };
        let value = value as f64;
        let prev = c.watched.values.get(name).copied();
        if prev != Some(value) {
            events.push((name.clone(), value, prev));
        }
    }
    for (name, value, _) in &events {
        c.watched.values.insert(name.clone(), *value);
    }

    for (name, value, prev) in events {
        let args = [
            Value::Str(name.clone()),
            Value::Float(value),
            Value::Float(prev.unwrap_or(value)),
        ];
        fire(c, &source, "onValueChange", &args);
        // The edge callbacks are the ones worth having: "the beat landed" is
        // a different question from "the number moved".
        let Some(prev) = prev else { continue };
        let t = threshold as f64;
        if prev < t && value >= t {
            fire(
                c,
                &source,
                "onOffToOn",
                &[Value::Str(name.clone()), Value::Float(value)],
            );
        } else if prev >= t && value < t {
            fire(
                c,
                &source,
                "onOnToOff",
                &[Value::Str(name), Value::Float(value)],
            );
        }
    }
    callback_dat_view(c, &source)
}

pub(crate) fn cook_par_execute(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    if !c.b("active") {
        return callback_dat_view(c, &source);
    }
    if !c.b("valuechange") {
        return callback_dat_view(c, &source);
    }
    let Some(watch_id) = resolve_dat_op(c, &c.s("op")) else {
        return callback_dat_view(c, &source);
    };
    let watch_path = c.graph.path(watch_id);
    let pattern = c.s("parameters");
    let names: Vec<String> = pattern
        .split_whitespace()
        .filter(|n| *n != "*")
        .map(|s| s.to_string())
        .collect();
    if names.is_empty() {
        c.error = Some("name the parameters to watch — `*` cannot be enumerated".into());
        return callback_dat_view(c, &source);
    }

    let mut events = Vec::new();
    for name in &names {
        let Some(value) = c
            .graph
            .get(watch_id)
            .and_then(|n| n.param(name))
            .map(|p| {
                p.eval(&EvalContext {
                    channels: c.eval.channels,
                    path: Some(&watch_path),
                    ..*c.eval
                })
            })
        else {
            continue;
        };
        let value = value.as_f64();
        let prev = c.watched.values.get(name).copied();
        if prev != Some(value) {
            events.push((name.clone(), value, prev));
        }
    }
    for (name, value, _) in &events {
        c.watched.values.insert(name.clone(), *value);
    }
    for (name, value, prev) in events {
        fire(
            c,
            &source,
            "onValueChange",
            &[
                Value::Str(name),
                Value::Float(value),
                Value::Float(prev.unwrap_or(value)),
            ],
        );
    }
    callback_dat_view(c, &source)
}

pub(crate) fn params_webclient() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "method" => Param::menu("GET", &["GET", "POST", "PUT", "DELETE"]).with_label("Method"),
        "url" => Param::str("https://httpbin.org/get").with_label("URL"),
        "headers" => Param::str("").with_label("Headers"),
        "body" => Param::str("").with_label("Request Body"),
        "fetch" => Param::bool(false).with_label("Fetch"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(WEBCLIENT_CALLBACKS).with_label("Source").into_script(),
    }
}

fn webclient_request_key(c: &DatCtx) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        c.s("method"),
        c.s("url"),
        c.s("body"),
        c.s("headers"),
    )
}

fn webclient_is_http(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://") || u.starts_with("https://")
}

fn webclient_display(slot: &crate::net::WebClient) -> DatData {
    if let Some(err) = &slot.error {
        return DatData::text(err.clone());
    }
    DatData::text(slot.body.clone())
}

pub(crate) fn cook_webclient(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    let url = c.s("url");
    let path = c.path.to_string();

    if !c.b("active") {
        if let Some(slot) = c.net.webclient.get(&path) {
            return webclient_display(slot);
        }
        return DatData::text("");
    }

    if webclient_is_http(&url) {
        let request_key = webclient_request_key(c);
        let method = c.s("method");
        let body = c.s("body");
        let headers = c.s("headers");
        let fetch = c.b("fetch");
        let prev_fetch = c
            .watched
            .values
            .get("_fetch")
            .copied()
            .unwrap_or(0.0)
            > 0.0;
        c.watched
            .values
            .insert("_fetch".into(), if fetch { 1.0 } else { 0.0 });
        let pulse = fetch && !prev_fetch;

        {
            let slot = c.net.webclient.entry(path.clone()).or_default();
            crate::net::webclient_poll(slot);
            if pulse {
                slot.fetch_gen += 1;
                let key = format!("{request_key}\0{}", slot.fetch_gen);
                crate::net::webclient_start_fetch(
                    slot,
                    key,
                    method,
                    url.clone(),
                    body,
                    headers,
                );
            }
        }

        if let Some(slot) = c.net.webclient.get(&path) {
            if slot.status >= 400 && slot.error.is_none() {
                c.error = Some(format!("HTTP {}", slot.status));
            } else if let Some(e) = &slot.error {
                c.error = Some(e.clone());
            }
        }

        let notify = c.net.webclient.get(&path).and_then(|slot| {
            if slot.error.is_none()
                && !slot.body.is_empty()
                && slot.notified_body.as_deref() != Some(&slot.body)
            {
                Some(slot.body.clone())
            } else {
                None
            }
        });
        if let Some(message) = notify {
            if let Some(s) = c.net.webclient.get_mut(&path) {
                s.notified_body = Some(message.clone());
            }
            fire(c, &source, "onMessage", &[Value::Str(message)]);
        }
        if let Some(slot) = c.net.webclient.get(&path) {
            return webclient_display(slot);
        }
        return DatData::text("");
    }

    if !c.watched.started {
        c.watched.started = true;
        let dat_path = Value::Str(c.path.to_string());
        fire(c, &source, "onConnect", std::slice::from_ref(&dat_path));
    }
    DatData::text(format!(
        "WebSocket is not implemented yet.\n{}",
        url.trim()
    ))
}

pub(crate) fn params_webserver() -> IndexMap<String, Param> {
    params! {
        "active" => Param::bool(true).with_label("Active"),
        "port" => Param::int(9980).with_label("Port").with_range(1.0, 65535.0),
        "localaddress" => Param::str("").with_label("Local Address"),
        "callbacks" => Param::str("").with_label("Callbacks DAT").as_path_ref(),
        "source" => Param::str(WEBSERVER_CALLBACKS).with_label("Source").into_script(),
    }
}

fn webserver_status_table(port: u16, address: &str, requests: u64) -> DatData {
    DatData::table(vec![
        vec!["name".into(), "value".into()],
        vec!["port".into(), port.to_string()],
        vec!["address".into(), address.to_string()],
        vec!["requests".into(), requests.to_string()],
    ])
}

pub(crate) fn cook_webserver(c: &mut DatCtx) -> DatData {
    let source = c.callback_script();
    let port = c.val("port").as_i64().clamp(1, 65535) as u16;
    let bind = c.s("localaddress");
    let path = c.path.to_string();
    let dat_name = c.node.name.clone();
    let active = c.b("active");

    if !active {
        c.net.webserver.remove(&path);
        if c.watched.started {
            c.watched.started = false;
            if let Some(host) = c.scripts {
                let script_path = dat_script_path(c);
                let _ = host.webserver_event(
                    &source,
                    "onServerStop",
                    &c.node.name,
                    c.eval,
                    &script_path,
                );
            }
        }
        c.watched.values.insert("_requests".into(), 0.0);
        return callback_dat_view(c, &source);
    }

    let key = crate::webserver::server_key(port, &bind);
    let needs_open = c
        .net
        .webserver
        .get(&path)
        .map(|s| crate::webserver::server_key(s.port, &s.bind) != key)
        .unwrap_or(true);
    if needs_open {
        c.net.webserver.remove(&path);
        match crate::webserver::WebServer::start(port, &bind) {
            Ok(srv) => {
                let bound_port = srv.port;
                let local = srv.local_address.clone();
                c.net.webserver.insert(path.clone(), srv);
                c.watched.values.insert("_requests".into(), 0.0);
                if !c.watched.started {
                    c.watched.started = true;
                    if let Some(host) = c.scripts {
                        let script_path = dat_script_path(c);
                        let _ = host.webserver_event(
                            &source,
                            "onServerStart",
                            &dat_name,
                            c.eval,
                            &script_path,
                        );
                    }
                }
                let req_count = c.watched.values.get("_requests").copied().unwrap_or(0.0) as u64;
                return webserver_status_table(bound_port, &local, req_count);
            }
            Err(e) => {
                c.error = Some(e);
                return callback_dat_view(c, &source);
            }
        }
    }

    let (bound_port, local, jobs, ws_events) = {
        let Some(server) = c.net.webserver.get(&path) else {
            return callback_dat_view(c, &source);
        };
        (
            server.port,
            server.local_address.clone(),
            server.drain_pending(),
            server.drain_ws_events(),
        )
    };

    let Some(host) = c.scripts else {
        for job in jobs {
            let _ = job.respond.send(crate::webserver::HttpReply {
                status: 503,
                reason: "Service Unavailable".into(),
                body: "Python is not available".into(),
                content_type: Some("text/plain".into()),
            });
        }
        c.error = Some("no Python interpreter in this build".into());
        let req_count = c.watched.values.get("_requests").copied().unwrap_or(0.0) as u64;
        return webserver_status_table(bound_port, &local, req_count);
    };

    for job in jobs {
        let script_path = dat_script_path(c);
        match host.webserver_http(&source, &dat_name, &job, c.eval, &script_path) {
            Ok(reply) => {
                let _ = job.respond.send(reply);
                let n = c.watched.values.get("_requests").copied().unwrap_or(0.0) + 1.0;
                c.watched.values.insert("_requests".into(), n);
            }
            Err(e) => {
                let _ = job.respond.send(crate::webserver::HttpReply {
                    status: 500,
                    reason: "Internal Server Error".into(),
                    body: e.clone(),
                    content_type: Some("text/plain".into()),
                });
                if c.error.is_none() {
                    c.error = Some(e);
                }
            }
        }
    }

    let script_path = dat_script_path(c);
    for ev in ws_events {
        let func = match &ev {
            crate::webserver::WebSocketEvent::Open { .. } => "onWebSocketOpen",
            crate::webserver::WebSocketEvent::Close { .. } => "onWebSocketClose",
            crate::webserver::WebSocketEvent::Text { .. } => "onWebSocketReceiveText",
            crate::webserver::WebSocketEvent::Binary { .. } => "onWebSocketReceiveBinary",
            crate::webserver::WebSocketEvent::Ping { .. } => "onWebSocketReceivePing",
            crate::webserver::WebSocketEvent::Pong { .. } => "onWebSocketReceivePong",
        };
        let client = match &ev {
            crate::webserver::WebSocketEvent::Open { client, .. }
            | crate::webserver::WebSocketEvent::Close { client }
            | crate::webserver::WebSocketEvent::Text { client, .. }
            | crate::webserver::WebSocketEvent::Binary { client, .. }
            | crate::webserver::WebSocketEvent::Ping { client, .. }
            | crate::webserver::WebSocketEvent::Pong { client, .. } => client.as_str(),
        };
        let uri = match &ev {
            crate::webserver::WebSocketEvent::Open { uri, .. } => Some(uri.as_str()),
            _ => None,
        };
        let text = match &ev {
            crate::webserver::WebSocketEvent::Text { data, .. } => Some(data.as_str()),
            _ => None,
        };
        let binary = match &ev {
            crate::webserver::WebSocketEvent::Binary { data, .. }
            | crate::webserver::WebSocketEvent::Ping { data, .. }
            | crate::webserver::WebSocketEvent::Pong { data, .. } => Some(data.as_slice()),
            _ => None,
        };
        if let Err(e) = host.webserver_ws(
            &source,
            func,
            &dat_name,
            client,
            uri,
            text,
            binary,
            c.eval,
            &script_path,
        ) {
            if c.error.is_none() {
                c.error = Some(e);
            }
        }
    }

    if let Some(server) = c.net.webserver.get(&path) {
        let sends = otd_core::take_queued_ws_sends_for(&path);
        server.apply_ws_sends(&sends);
    }

    let req_count = c.watched.values.get("_requests").copied().unwrap_or(0.0) as u64;
    webserver_status_table(bound_port, &local, req_count)
}

// ------------------------------------------------------------------ sort

pub(crate) fn params_sort() -> IndexMap<String, Param> {
    params! {
        "column" => Param::str("0").with_label("Column (name or index)"),
        "order" => Param::menu("ascending", &["ascending", "descending"]).with_label("Order"),
        "numeric" => Param::bool(true).with_label("Compare As Numbers"),
        "header" => Param::bool(true).with_label("First Row Is A Header"),
    }
}

fn column_index(rows: &[Vec<String>], spec: &str, header: bool) -> usize {
    if let Ok(i) = spec.trim().parse::<usize>() {
        return i;
    }
    if header {
        if let Some(head) = rows.first() {
            if let Some(i) = head.iter().position(|c| c == spec.trim()) {
                return i;
            }
        }
    }
    0
}

pub(crate) fn cook_sort(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let header = c.b("header");
    let col = column_index(&input.rows, &c.s("column"), header);
    let numeric = c.b("numeric");
    let descending = c.menu("order") == 1;

    let mut rows = input.rows.clone();
    let head = if header && !rows.is_empty() {
        Some(rows.remove(0))
    } else {
        None
    };
    rows.sort_by(|a, b| {
        let (x, y) = (
            a.get(col).map(|s| s.as_str()).unwrap_or(""),
            b.get(col).map(|s| s.as_str()).unwrap_or(""),
        );
        let ord = if numeric {
            // A cell that is not a number sorts as if it were zero rather
            // than falling back to string order for that one row, which
            // would interleave the two orderings unpredictably.
            let (x, y) = (
                x.trim().parse::<f64>().unwrap_or(0.0),
                y.trim().parse::<f64>().unwrap_or(0.0),
            );
            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            x.cmp(y)
        };
        if descending { ord.reverse() } else { ord }
    });
    if let Some(head) = head {
        rows.insert(0, head);
    }
    DatData::table(rows)
}

// ------------------------------------------------------------- transpose

pub(crate) fn cook_transpose(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let cols = input.num_cols();
    let rows = (0..cols)
        .map(|col| {
            input
                .rows
                .iter()
                .map(|r| r.get(col).cloned().unwrap_or_default())
                .collect()
        })
        .collect();
    DatData::table(rows)
}

// ------------------------------------------------------------ substitute

pub(crate) fn params_substitute() -> IndexMap<String, Param> {
    params! {
        "template" => Param::str("").with_label("Template"),
        "table" => Param::bool(false).with_label("Substitute Every Cell Instead"),
    }
}

/// Fill `$name` placeholders from a two-column lookup table.
///
/// This is the operator that turns a cue table into a string somebody else
/// can read — an OSC address, a UDP payload, a line of a show report — without
/// a script. The input's first column names, the second supplies.
pub(crate) fn cook_substitute(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let substitute = |text: &str| -> String {
        let mut out = text.to_string();
        for row in &input.rows {
            let (Some(key), Some(value)) = (row.first(), row.get(1)) else {
                continue;
            };
            if key.trim().is_empty() {
                continue;
            }
            out = out.replace(&format!("${key}"), value);
        }
        out
    };

    if !c.b("table") {
        return DatData::text(substitute(&c.s("template")));
    }
    DatData::table(
        input
            .rows
            .iter()
            .map(|r| r.iter().map(|cell| substitute(cell)).collect())
            .collect(),
    )
}

// --------------------------------------------------------------- convert

pub(crate) fn params_convert() -> IndexMap<String, Param> {
    params! {
        "to" => Param::menu("table", &["table", "text"]).with_label("Convert To"),
        "delimiter" => Param::menu("tab", &["tab", "comma"]).with_label("Delimiter"),
    }
}

/// Between the two shapes a DAT can have.
///
/// They are one representation internally — a text DAT is a table with one
/// cell — but the operators reading them are not interchangeable, so the
/// conversion has to be something you can put in a patch rather than a flag
/// hidden on every node.
pub(crate) fn cook_convert(c: &mut DatCtx) -> DatData {
    let input = c.input(0);
    let delim = if c.menu("delimiter") == 1 { ',' } else { '\t' };
    if c.menu("to") == 1 {
        let text = input
            .rows
            .iter()
            .map(|r| r.join(&delim.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        return DatData::text(text);
    }
    DatData::from_delimited(&input.as_text(), delim)
}

// ------------------------------------------------------- CHOP to DAT

pub(crate) fn params_chop_to() -> IndexMap<String, Param> {
    params! {
        "layout" => Param::menu("columns", &["columns", "rows"])
            .with_label("Channels As"),
        "names" => Param::bool(true).with_label("Include Names"),
        "format" => Param::str("%.6g").with_label("Number Format"),
    }
}

/// Channels as a table — the readable end of a CHOP.
///
/// Mostly this is how you *look* at a CHOP: wire one in and the numbers are
/// text you can select and copy. It is also how a channel leaves the patch,
/// since a UDP Out DAT sends whatever text reaches it, so "send my six
/// smoothed values to the lighting desk" is two nodes rather than a feature.
pub(crate) fn cook_chop_to_dat(c: &mut DatCtx) -> DatData {
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

/// `%.6g`-style formatting without pulling in a formatting crate: the only
/// thing that actually varies is how many digits to keep.
fn format_number(spec: &str, v: f32) -> String {
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
        // Trailing zeros are noise in a table you are reading.
        let s = format!("{:.*}", digits, v);
        let s = if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s
        };
        if s == "-0" { "0".into() } else { s }
    }
}
