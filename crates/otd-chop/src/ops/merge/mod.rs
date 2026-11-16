use crate::data::{CONTROL_RATE, ChopData};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let mut channels = Vec::new();
    let mut len = 0usize;
    let mut rate = CONTROL_RATE;
    let mut time_sliced = false;
    for input in &c.inputs {
        if input.channels.is_empty() {
            continue;
        }
        len = len.max(input.num_samples());
        rate = rate.max(input.sample_rate);
        time_sliced |= input.time_sliced;
        channels.extend(input.channels.iter().cloned());
    }

    // Duplicate names would make an Export ambiguous, so number them.
    let mut seen: Vec<String> = Vec::new();
    for ch in &mut channels {
        if seen.contains(&ch.name) {
            let mut i = 1;
            loop {
                let candidate = format!("{}{}", ch.name, i);
                if !seen.contains(&candidate) {
                    ch.name = candidate;
                    break;
                }
                i += 1;
            }
        }
        seen.push(ch.name.clone());
    }
    let mut out = ChopData::new(channels, rate, time_sliced);
    out.pad_to(len);
    out
}



pub const TYPE_NAME: &str = "mergeCHOP";

pub fn spec() -> ChopSpec {
    super::spec(
                TYPE_NAME,
                "Merge",
                &["a", "b"],
                "Put two CHOPs' channels side by side.",
                no_params,
                cook,
            )
}
