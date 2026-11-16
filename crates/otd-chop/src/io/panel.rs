//! I/O submodule: panel.

use crate::ops::ChopCtx;

use super::*;

// ------------------------------------------------------- mouse, keyboard

pub(crate) fn params_mouse_in() -> IndexMap<String, Param> {
    IndexMap::new()
}

pub(crate) fn cook_mouse_in(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let s = c.io.input.clone();
    ChopData::new(
        vec![
            Channel::constant("tx", s.mouse[0], n),
            Channel::constant("ty", s.mouse[1], n),
            Channel::constant("button1", s.buttons[0] as i32 as f32, n),
            Channel::constant("button2", s.buttons[1] as i32 as f32, n),
            Channel::constant("button3", s.buttons[2] as i32 as f32, n),
            Channel::constant("wheel", s.wheel, n),
        ],
        rate,
        true,
    )
}

pub(crate) fn params_keyboard_in() -> IndexMap<String, Param> {
    let mut m = IndexMap::new();
    m.insert(
        "keys".into(),
        Param::str("1 a s d f").with_label("Keys (space separated)"),
    );
    m
}

fn keyboard_channel(key: &str) -> String {
    let k = key.trim().to_ascii_lowercase();
    if k.starts_with('k') && k.len() > 1 {
        k
    } else {
        format!("k{k}")
    }
}

fn egui_key_token(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("num") {
        if rest.len() == 1 && rest.chars().all(|c| c.is_ascii_digit()) {
            return rest.to_string();
        }
    }
    name.to_string()
}

fn key_held(held: &[String], token: &str) -> bool {
    let token = token.trim().to_ascii_lowercase();
    held.iter().any(|h| {
        let h = h.to_ascii_lowercase();
        h == token || egui_key_token(&h) == token
    })
}

pub(crate) fn cook_keyboard_in(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let keys = c
        .node
        .param("keys")
        .map(|p| p.eval(c.eval).as_str())
        .unwrap_or_default();
    let held = c.io.input.keys.clone();
    let channels: Vec<Channel> = keys
        .split_whitespace()
        .map(|k| {
            let down = key_held(&held, k);
            Channel::constant(keyboard_channel(k), down as i32 as f32, n)
        })
        .collect();
    let channels = if channels.is_empty() {
        vec![Channel::constant("chan1", 0.0, n)]
    } else {
        channels
    };
    ChopData::new(channels, rate, true)
}

pub(crate) fn params_perform_in() -> IndexMap<String, Param> {
    IndexMap::new()
}

pub(crate) fn cook_perform_in(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let p = c.io.perform_panel;
    ChopData::new(
        vec![
            Channel::constant("u", p.u, n),
            Channel::constant("v", p.v, n),
            Channel::constant("lbutton", p.lbutton as i32 as f32, n),
            Channel::constant("click", p.click as i32 as f32, n),
            Channel::constant("release", p.release as i32 as f32, n),
            Channel::constant("wheel", p.wheel, n),
            Channel::constant("inside", p.inside as i32 as f32, n),
        ],
        rate,
        true,
    )
}

