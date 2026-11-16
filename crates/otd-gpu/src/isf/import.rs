//!
//! PLAN.md §6 names this as the lever against the operator-breadth treadmill:
//! ISF is a published format with thousands of existing effects, and every
//! one of them is a GLSL fragment shader with a JSON header describing its
//! inputs. Importing one turns it into an ordinary GLSL TOP whose inputs are
//! ordinary parameters — after which nothing in the system knows or cares
//! that it came from somewhere else.
//!
//! Measured against the published collection at Vidvox/ISF-Files — 327 real
//! shaders — rather than against shaders written for the test, which only
//! ever proves the happy path. `cargo run -p otd-gpu --example isf_corpus`
//! reports the current figure and groups what fails by why. It stands at 225
//! of 327, and the remaining buckets are: multi-pass (56, a real feature and
//! a real absence), shaders whose varyings come from a companion vertex
//! shader we do not run (17), names we do not define — persistent buffers
//! mostly, which are multi-pass by another route (18), and shaders wanting an
//! audio texture (5), which needs a sound source before it needs a shader
//! change.
//!
//! Uniform space used to be a bucket of sixteen and is now none: the block is
//! twelve `vec4`s wide, which is where the widest published shader fits. See
//! [`crate::ops::PARAM_VECS`].
//!
//! What is supported is what the format actually uses in practice: `float`,
//! `bool`, `long` (a menu), `color`, `point2D` and `image` inputs, the
//! `IMG_THIS_PIXEL`/`IMG_NORM_PIXEL`/`IMG_PIXEL` accessors, and the standard
//! `isf_FragNormCoord`/`TIME`/`RENDERSIZE` variables. Multi-pass ISF is not:
//! it needs several render targets, which is a Phase 6 concern.

use otd_core::indexmap::IndexMap;
use otd_core::{Graph, NodeId, Param, Value};

use super::codegen::{number_array, preamble, wrap_entry_point};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IsfError {
    #[error("no ISF header: the file must start with a /*{{ ... }}*/ JSON comment")]
    NoHeader,
    #[error("the ISF header is not valid JSON: {0}")]
    BadJson(String),
    #[error("multi-pass ISF shaders are not supported yet")]
    MultiPass,
    #[error("this shader needs more uniform space than a GLSL TOP has (12 vec4s)")]
    TooManyInputs,
}

/// An imported shader: the parameters it wants, and the GLSL to run.
#[derive(Debug, Clone)]
pub struct Isf {
    pub name: String,
    pub description: String,
    pub categories: Vec<String>,
    /// Parameters to put on the node, in the order the header declares them.
    pub params: IndexMap<String, Param>,
    /// Which uniform each parameter feeds, in declaration order. The engine
    /// packs them into `U.p0..p11`.
    pub inputs: Vec<IsfInput>,
    /// GLSL ready to hand to the GLSL TOP.
    pub source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IsfInput {
    pub name: String,
    pub kind: IsfKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsfKind {
    Float,
    Bool,
    Long,
    Color,
    Point2d,
    /// An image input, wired rather than dialled.
    Image,
}

impl IsfKind {
    /// Uniform components this kind occupies. Images occupy none — they are
    /// textures.
    pub fn width(self) -> usize {
        match self {
            IsfKind::Float | IsfKind::Bool | IsfKind::Long => 1,
            IsfKind::Point2d => 2,
            IsfKind::Color => 4,
            IsfKind::Image => 0,
        }
    }
}

/// VALUES for an ISF `long`, one per label. Missing entries are the index.
fn isf_long_values(entry: &serde_json::Value, len: usize) -> Vec<String> {
    let listed = entry
        .get("VALUES")
        .and_then(|v| v.as_array())
        .map(|vals| vals.iter().map(isf_json_scalar).collect::<Vec<_>>())
        .unwrap_or_default();
    (0..len)
        .map(|i| {
            listed
                .get(i)
                .filter(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| i.to_string())
        })
        .collect()
}

fn isf_json_scalar(v: &serde_json::Value) -> String {
    if let Some(n) = v.as_i64() {
        return n.to_string();
    }
    if let Some(n) = v.as_f64() {
        if n.fract() == 0.0 {
            return format!("{}", n as i64);
        }
        return n.to_string();
    }
    v.as_str().unwrap_or_default().to_string()
}

/// ISF's DEFAULT is the value when it matches one, otherwise an index.
fn isf_long_default(entry: &serde_json::Value, values: &[String]) -> String {
    let Some(d) = entry.get("DEFAULT") else {
        return values.first().cloned().unwrap_or_else(|| "0".into());
    };
    if let Some(s) = d.as_str() {
        return s.to_string();
    }
    if let Some(n) = d.as_i64() {
        let as_text = n.to_string();
        if values.iter().any(|v| v == &as_text) {
            return as_text;
        }
        if n >= 0 {
            if let Some(v) = values.get(n as usize) {
                return v.clone();
            }
        }
        return as_text;
    }
    values.first().cloned().unwrap_or_else(|| "0".into())
}

/// Split an ISF file into its JSON header and its GLSL body.
fn split(source: &str) -> Result<(&str, &str), IsfError> {
    let start = source.find("/*").ok_or(IsfError::NoHeader)?;
    let end = source[start..].find("*/").ok_or(IsfError::NoHeader)? + start;
    let header = source[start + 2..end].trim();
    // The header is a JSON object; some files wrap it in `{{ }}` and some do
    // not, so find the object rather than assuming.
    let obj_start = header.find('{').ok_or(IsfError::NoHeader)?;
    let obj_end = header.rfind('}').ok_or(IsfError::NoHeader)?;
    if obj_end <= obj_start {
        return Err(IsfError::NoHeader);
    }
    Ok((&header[obj_start..=obj_end], &source[end + 2..]))
}

/// Convert an ISF file into something the GLSL TOP can run.
pub fn import(source: &str) -> Result<Isf, IsfError> {
    let (header_text, body) = split(source)?;
    let header: serde_json::Value =
        serde_json::from_str(header_text).map_err(|e| IsfError::BadJson(e.to_string()))?;

    if header
        .get("PASSES")
        .and_then(|p| p.as_array())
        .map(|p| p.len() > 1)
        .unwrap_or(false)
    {
        return Err(IsfError::MultiPass);
    }

    let mut params = IndexMap::new();
    let mut inputs = Vec::new();
    let mut image_count = 0usize;

    if let Some(list) = header.get("INPUTS").and_then(|i| i.as_array()) {
        for entry in list {
            let Some(name) = entry.get("NAME").and_then(|n| n.as_str()) else {
                continue;
            };
            let kind = entry
                .get("TYPE")
                .and_then(|t| t.as_str())
                .unwrap_or("float");
            let label = entry
                .get("LABEL")
                .and_then(|l| l.as_str())
                .unwrap_or(name)
                .to_string();

            let (param, kind) = match kind {
                "bool" => (
                    Param::bool(
                        entry
                            .get("DEFAULT")
                            .and_then(|d| d.as_f64())
                            .map(|d| d != 0.0)
                            .unwrap_or(false),
                    ),
                    IsfKind::Bool,
                ),
                "long" => {
                    // A `long` is ISF's menu: LABELS are the options, VALUES
                    // are what the shader sees.
                    let labels: Vec<String> = entry
                        .get("LABELS")
                        .and_then(|l| l.as_array())
                        .map(|l| {
                            l.iter()
                                .map(|s| s.as_str().unwrap_or_default().to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    if labels.is_empty() {
                        (Param::int(0), IsfKind::Long)
                    } else {
                        let values = isf_long_values(entry, labels.len());
                        let choices: Vec<(&str, &str)> = labels
                            .iter()
                            .zip(values.iter())
                            .map(|(option, value)| (option.as_str(), value.as_str()))
                            .collect();
                        let default = isf_long_default(entry, &values);
                        (Param::select(&default, &choices), IsfKind::Long)
                    }
                }
                "color" => {
                    let d = number_array(entry.get("DEFAULT"), 4, [1.0, 1.0, 1.0, 1.0]);
                    (Param::rgba(d), IsfKind::Color)
                }
                "point2D" => {
                    let d = number_array(entry.get("DEFAULT"), 2, [0.0, 0.0, 0.0, 0.0]);
                    (
                        Param::new(otd_core::Value::Vec2([d[0], d[1]])),
                        IsfKind::Point2d,
                    )
                }
                "image" => {
                    image_count += 1;
                    (Param::str(""), IsfKind::Image)
                }
                _ => {
                    let default = entry.get("DEFAULT").and_then(|d| d.as_f64()).unwrap_or(0.0);
                    let min = entry.get("MIN").and_then(|d| d.as_f64());
                    let max = entry.get("MAX").and_then(|d| d.as_f64());
                    let p = Param::float(default);
                    let p = match (min, max) {
                        (Some(lo), Some(hi)) if hi > lo => p.with_range(lo, hi),
                        _ => p,
                    };
                    (p, IsfKind::Float)
                }
            };

            // Image inputs are wires, not dials; they do not appear on the
            // parameter page.
            if kind != IsfKind::Image {
                params.insert(name.to_string(), param.with_label(&label));
            }
            inputs.push(IsfInput {
                name: name.to_string(),
                kind,
            });
        }
    }

    Ok(Isf {
        name: header
            .get("CREDIT")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string(),
        description: header
            .get("DESCRIPTION")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string(),
        categories: header
            .get("CATEGORIES")
            .and_then(|c| c.as_array())
            .map(|c| {
                c.iter()
                    .filter_map(|s| s.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        source: {
            let (prelude, assignments) = preamble(&inputs, image_count)?;
            prelude + &wrap_entry_point(body, &assignments)
        },
        params,
        inputs,
    })
}

/// Load an imported shader onto an existing GLSL TOP.
///
/// The node keeps being an ordinary `glslTOP` afterwards: the shader is in its
/// `source` parameter and the ISF inputs are custom parameters like any other.
/// Nothing downstream knows the difference, which is the whole point — an
/// imported effect can be parameter-bound, exported to, animated and saved
/// exactly like a hand-written one.
pub fn apply(graph: &mut Graph, id: NodeId, isf: &Isf) {
    // Drop any parameters a previous import left behind, so re-importing a
    // different shader onto the same node does not accumulate dead dials.
    let stale: Vec<String> = graph
        .node(id)
        .params
        .iter()
        .filter(|(k, p)| p.custom && !isf.params.contains_key(k.as_str()))
        .map(|(k, _)| k.clone())
        .collect();
    for key in stale {
        graph.remove_custom_param(id, &key);
    }

    let _ = graph.set_param(id, "language", Value::Str("glsl".into()));
    let _ = graph.set_param(id, "source", Value::Str(isf.source.clone()));
    for (key, param) in &isf.params {
        graph.add_custom_param(id, key, param.clone());
    }
}

/// Where one parameter lands in the `U.p0..p11` uniform block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Slot {
    /// Which `U.p*` — 0..`ops::PARAM_VECS`.
    pub vec: usize,
    /// First component within it — 0..3, i.e. `.x`..`.w`.
    pub component: usize,
    /// How many components it occupies: 1, 2 or 4.
    pub width: usize,
}

impl Slot {
    /// The GLSL swizzle that selects this slot, e.g. `.zw` or nothing at all
    /// for a full vector.
    pub fn swizzle(&self) -> &'static str {
        const NAMES: [&str; 4] = ["x", "y", "z", "w"];
        match (self.width, self.component) {
            (4, _) => "",
            (2, 0) => ".xy",
            (2, 2) => ".zw",
            (_, c) => match NAMES[c] {
                "x" => ".x",
                "y" => ".y",
                "z" => ".z",
                _ => ".w",
            },
        }
    }
}

