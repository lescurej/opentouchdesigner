use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData};

use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "threshold" => Param::float(0.5).with_label("Threshold"),
        "attack" => Param::float(0.05).with_label("Attack (s)").with_range(0.0, 2.0),
        "decay" => Param::float(0.2).with_label("Decay (s)").with_range(0.0, 4.0),
        "sustain" => Param::float(0.0).with_label("Sustain Level").with_range(0.0, 1.0),
        "release" => Param::float(0.3).with_label("Release (s)").with_range(0.0, 4.0),
    }
}

const STAGE_IDLE: i64 = 0;
const STAGE_ATTACK: i64 = 1;
const STAGE_SUSTAIN: i64 = 2;
const STAGE_RELEASE: i64 = 3;

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let input = c.input(0).clone();
    let rate = input.sample_rate.max(1.0);
    let dt = 1.0 / rate as f32;
    let (threshold, attack, decay, sustain, release) = (
        c.f("threshold"),
        c.f("attack").max(1e-4),
        c.f("decay").max(1e-4),
        c.f("sustain").clamp(0.0, 1.0),
        c.f("release").max(1e-4),
    );
    c.state.fit(input.num_channels());

    let mut channels = Vec::with_capacity(input.num_channels());
    for (ci, ch) in input.channels.iter().enumerate() {
        let mut level = c.state.f[ci];
        let mut stage = c.state.i[ci];
        let mut prev = c.state.prev[ci];
        let samples: Vec<f32> = ch
            .samples
            .iter()
            .map(|x| {
                // Rising edge through the threshold starts the envelope.
                if prev <= threshold && *x > threshold {
                    stage = STAGE_ATTACK;
                } else if prev > threshold && *x <= threshold && stage != STAGE_IDLE {
                    stage = STAGE_RELEASE;
                }
                prev = *x;

                match stage {
                    STAGE_ATTACK => {
                        level += dt / attack;
                        if level >= 1.0 {
                            level = 1.0;
                            stage = STAGE_SUSTAIN;
                        }
                    }
                    STAGE_SUSTAIN => {
                        if level > sustain {
                            level -= dt / decay * (1.0 - sustain).max(1e-6);
                            level = level.max(sustain);
                        }
                    }
                    STAGE_RELEASE => {
                        level -= dt / release;
                        if level <= 0.0 {
                            level = 0.0;
                            stage = STAGE_IDLE;
                        }
                    }
                    _ => {}
                }
                level
            })
            .collect();
        c.state.f[ci] = level;
        c.state.i[ci] = stage;
        c.state.prev[ci] = prev;
        channels.push(Channel::new(ch.name.clone(), samples));
    }
    ChopData::new(channels, input.sample_rate, input.time_sliced)
}



pub const TYPE_NAME: &str = "triggerCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Trigger",
                &["in"],
                "An attack/decay/sustain/release envelope per channel.",
                params,
                cook,
            )
}
