use otd_core::indexmap::IndexMap;
use otd_core::Param;

use crate::data::{Channel, ChopData, CONTROL_RATE};

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

/// Copy an input's shape, applying `f` to every sample of every channel.
pub(crate) fn map_samples(input: &ChopData, mut f: impl FnMut(usize, usize, f32) -> f32) -> ChopData {
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
