use otd_core::indexmap::IndexMap;
use otd_core::{Param, Value};

use crate::DatData;

use super::callbacks::fire;
use super::constants::*;
use super::{DatCtx, DatSpec, spec as dat_spec};

pub(crate) fn params() -> IndexMap<String, Param> {
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

pub(crate) fn cook(c: &mut DatCtx) -> DatData {
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
        let prev_fetch = c.watched.values.get("_fetch").copied().unwrap_or(0.0) > 0.0;
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
                crate::net::webclient_start_fetch(slot, key, method, url.clone(), body, headers);
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
    DatData::text(format!("WebSocket is not implemented yet.\n{}", url.trim()))
}
pub const TYPE_NAME: &str = "webclientDAT";

pub fn spec() -> DatSpec {
    let mut s = dat_spec(
        TYPE_NAME,
        "Web Client",
        &[],
        "HTTP client DAT.",
        params,
        cook,
    );
    s.def.time_dependent = true;
    s
}
