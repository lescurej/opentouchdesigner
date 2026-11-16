use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{Channel, ChopData, slice_len};

use super::helpers::*;
use super::{ChopCtx, ChopSpec};


pub(crate) fn params() -> IndexMap<String, Param> {
    with_rate(params! {
        "length" => Param::float(10.0).with_label("Length (s)").with_range(0.1, 600.0),
        "cycle" => Param::bool(true).with_label("Cycle"),
        "play" => Param::bool(true).with_label("Play"),
    })
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let rate = c.rate();
    let n = slice_len(rate, c.time);
    let dt = (c.time.dt / n as f64) as f32;
    let (length, cycle, play) = (c.f("length").max(1e-3), c.b("cycle"), c.b("play"));
    c.state.fit(1);

    let mut elapsed = c.state.f[0];
    let mut cycles = c.state.i[0];
    let mut done = 0.0f32;

    let (mut fraction, mut seconds, mut cyc, mut don) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    for _ in 0..n {
        if play {
            elapsed += dt;
            if elapsed >= length {
                if cycle {
                    elapsed -= length;
                    cycles += 1;
                } else {
                    elapsed = length;
                    done = 1.0;
                }
            }
        }
        fraction.push(elapsed / length);
        seconds.push(elapsed);
        cyc.push(cycles as f32);
        don.push(done);
    }
    c.state.f[0] = elapsed;
    c.state.i[0] = cycles;

    ChopData::new(
        vec![
            Channel::new("timer_fraction", fraction),
            Channel::new("timer_seconds", seconds),
            Channel::new("cycles", cyc),
            Channel::new("done", don),
        ],
        rate,
        true,
    )
}



pub const TYPE_NAME: &str = "timerCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME,
                "Timer",
                &[],
                "A running timer with fraction, seconds, cycles and done.",
                params,
                cook,
            )
}
