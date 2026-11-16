use otd_core::Param;
use otd_core::indexmap::IndexMap;
use otd_core::NodeId;

use crate::data::{CONTROL_RATE, Channel, ChopData};
use crate::ops::ChopCtx;

macro_rules! params {
    ($($key:expr => $param:expr),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut m: IndexMap<String, Param> = IndexMap::new();
        $( m.insert($key.into(), $param); )*
        m
    }};
}

pub(crate) fn no_params() -> IndexMap<String, Param> {
    params! {}
}

pub(crate) fn with_rate(mut m: IndexMap<String, Param>) -> IndexMap<String, Param> {
    m.insert(
        "rate".into(),
        Param::float(CONTROL_RATE)
            .with_label("Sample Rate")
            .with_range(1.0, 48000.0),
    );
    m
}

pub(crate) fn chop_common_page_params() -> IndexMap<String, Param> {
    params! {
        "timeslice" => Param::bool(false).with_label("Time Slice").with_page("Common"),
        "scope" => Param::str("*").with_label("Scope").with_page("Common"),
        "srselect" => Param::menu("max", &["first", "max", "min", "err"])
            .with_label("Sample Rate Match")
            .with_page("Common"),
    }
}

pub(crate) fn merge_param_maps(
    mut base: IndexMap<String, Param>,
    extra: IndexMap<String, Param>,
) -> IndexMap<String, Param> {
    for (k, v) in extra {
        base.entry(k).or_insert(v);
    }
    base
}

/// Copy an input's shape, applying `f` to every sample of every channel.
pub(crate) fn resolve_op_path(c: &ChopCtx, reference: &str) -> Option<NodeId> {
    let path = reference.trim();
    if path.is_empty() {
        return None;
    }
    if path.starts_with('/') {
        c.graph.find(path)
    } else {
        c.graph.find_from(c.id, path)
    }
}

pub(crate) fn wired_or_referenced_chop(c: &ChopCtx, wire: usize, chop_param: &str) -> ChopData {
    let wired = c
        .inputs
        .get(wire)
        .filter(|d| !d.channels.is_empty())
        .map(|a| a.as_ref().clone());
    if let Some(data) = wired {
        return data;
    }
    let reference = c.s(chop_param);
    if otd_core::operator_reference_is_pattern(&reference) {
        if let Some(id) = c
            .graph
            .chops_matching_reference(c.id, &reference)
            .into_iter()
            .next()
        {
            if let Some(data) = c.store.get(id) {
                return data.clone();
            }
        }
    } else if let Some(id) = resolve_op_path(c, &reference) {
        if let Some(data) = c.store.get(id) {
            return data.clone();
        }
    }
    c.input(wire).clone()
}

pub(crate) fn script_param(c: &ChopCtx, key: &str) -> String {
    c.node
        .param(key)
        .map(|p| p.text_for_cook())
        .unwrap_or_default()
}

pub(crate) fn map_samples(
    input: &ChopData,
    mut f: impl FnMut(usize, usize, f32) -> f32,
) -> ChopData {
    let channels = input
        .channels
        .iter()
        .enumerate()
        .map(|(ci, c)| Channel {
            name: c.name.clone(),
            samples: c
                .samples
                .iter()
                .enumerate()
                .map(|(si, v)| f(ci, si, *v))
                .collect(),
        })
        .collect();
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}
