use serde::{Deserialize, Serialize};

use crate::expr::Expr;
use crate::value::Value;

pub fn channel_expression(op_path: &str, channel: &str) -> String {
    format!("ch('{op_path}', '{channel}')")
}

pub fn parameter_expression(op_path: &str, param: &str) -> String {
    format!("par('{op_path}', '{param}')")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectOption {
    pub option: String,
    pub value: String,
}

impl SelectOption {
    pub fn new(option: &str, value: &str) -> Self {
        Self {
            option: option.to_string(),
            value: value.to_string(),
        }
    }

    pub fn same(s: &str) -> Self {
        Self::new(s, s)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioDeviceList {
    #[default]
    None,
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamMode {
    #[default]
    Constant,
    Expression,
    Export,
    Bind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Param {
    /// Human label for the parameter panel. Falls back to the key if empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Inspector tab (TouchDesigner-style pages). Definition-only, not stored in `.otd`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub page: String,
    /// The constant value, and the declared type of the parameter.
    pub value: Value,
    #[serde(default, skip_serializing_if = "is_default_mode")]
    pub mode: ParamMode,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub expression: String,
    /// `op_path:channel` for Export mode, `op_path:param` for Bind mode.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// Soft UI range for sliders. Does not clamp the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<(f64, f64)>,
    /// Menu entries, if this parameter is a menu.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub menu: Option<Vec<SelectOption>>,
    /// Log-scale UI for float parameters (operator metadata, not saved in `.otd`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub logarithmic: bool,
    /// Low end of a log slider when `logarithmic` is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_floor: Option<f64>,
    /// Added by the author rather than by the operator definition. Custom
    /// parameters on a component are that component's API (PLAN.md §2.3), so
    /// they are written to the project file in full — nothing else knows they
    /// exist.
    #[serde(default, skip_serializing_if = "is_false")]
    pub custom: bool,
    /// Custom parameter that stores an operator path (`path_ref` on disk).
    #[serde(default, skip_serializing_if = "is_false")]
    pub op_ref: bool,

    #[serde(skip)]
    compiled: Option<Expr>,
    #[serde(skip)]
    error: Option<String>,
    /// The built-in language could not parse this, so it is Python's.
    #[serde(skip)]
    needs_python: bool,
    /// Operator paths this expression mentions, so the cook engine can make
    /// them dependencies.
    #[serde(skip)]
    refs: Vec<String>,
    /// This parameter holds script source rather than a value, so the paths
    /// inside it are references even though the parameter is a constant.
    /// Set by the operator definition, not by the project file.
    #[serde(skip)]
    script: bool,
    /// This parameter's whole value is an operator path — a Render TOP's
    /// camera, a Geometry COMP's SOP. Those have to cook first, so the path
    /// is a dependency exactly like a wire.
    #[serde(skip)]
    path_ref: bool,
    /// This parameter's value is a path to a file on disk — a movie, an
    /// image, an audio stem. Not a cook dependency, but an *export*
    /// dependency: a bundle has to bring it along.
    #[serde(skip)]
    file_ref: bool,
    /// Directory picker rather than a single file.
    #[serde(skip)]
    folder_ref: bool,
    /// Fill the menu from live audio devices when the UI opens.
    #[serde(skip)]
    audio_device_list: AudioDeviceList,
}

/// Pull operator paths out of an expression.
///
/// An expression like `ch('/lfo1', 'chan1')` has to make `/lfo1` cook first,
/// and there is no way to know that without looking at the source: the
/// interpreter only finds out when it runs. Quoted strings that look like
/// operator paths are treated as references. Over-reporting is harmless — an
/// extra dependency costs one cached lookup — while under-reporting would
/// read a stale channel.
fn extract_paths(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < bytes.len() {
        let quote = bytes[i];
        if quote != '\'' && quote != '"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len() && bytes[j] != quote {
            j += 1;
        }
        let literal: String = bytes[start..j.min(bytes.len())].iter().collect();
        if literal.starts_with('/') && !out.contains(&literal) {
            out.push(literal);
        }
        i = j + 1;
    }
    out
}

fn is_default_mode(m: &ParamMode) -> bool {
    *m == ParamMode::Constant
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A path parameter's whole value, when it looks like one.
fn own_path(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Vec::new()
    } else {
        vec![trimmed.to_string()]
    }
}

/// Keep the parameter's declared type: a Python expression returning a float
/// for an int parameter should round, not change the parameter's type.
fn coerce_like(declared: &Value, produced: Value) -> Value {
    if declared.same_type_as(&produced) {
        produced
    } else {
        declared.coerce_from_f64(produced.as_f64())
    }
}

include!("impl_builders.rs");
include!("impl_main.rs");
