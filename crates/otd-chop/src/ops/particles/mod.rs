use otd_core::indexmap::IndexMap;
use otd_core::Param;
use crate::data::{CONTROL_RATE, Channel, ChopData};

use super::{ChopCtx, ChopSpec};

pub(crate) fn params() -> IndexMap<String, Param> {
    params! {
        "count" => Param::int(256).with_label("Particle Count").with_range(1.0, 4096.0),
        "emitx" => Param::float(0.5).with_label("Emitter X (0..1)").with_range(0.0, 1.0),
        "emity" => Param::float(0.5).with_label("Emitter Y (0..1, down)").with_range(0.0, 1.0),
        "speed" => Param::float(0.2).with_label("Speed (frame widths/s)").with_range(0.0, 2.0),
        "gravity" => Param::float(0.2).with_label("Downward Acceleration").with_range(-2.0, 2.0),
        "lifetime" => Param::float(2.0).with_label("Lifetime (s)").with_range(0.1, 20.0),
        "active" => Param::bool(true).with_label("Emit"),
        "reset" => Param::bool(false).with_label("Reset"),
    }
}

pub(crate) fn cook(c: &mut ChopCtx) -> ChopData {
    let n = c.i("count").clamp(1, 4096) as usize;
    let lifetime = c.f("lifetime").clamp(0.1, 20.0);
    let (emitx, emity, speed, gravity) = (c.f("emitx"), c.f("emity"), c.f("speed"), c.f("gravity"));
    let active = c.b("active");
    let rewind = c.state.prev.first().is_some_and(|t| c.time.abs_time < *t as f64);
    if c.state.f.len() != n * 5 || c.b("reset") || rewind {
        c.state.f = vec![0.0; n * 5];
        for p in c.state.f.chunks_mut(5) { p[4] = lifetime; }
        c.state.i = vec![0; n];
    }
    c.state.prev = vec![c.time.abs_time as f32];
    let dt = c.time.dt.clamp(0.0, 0.1) as f32;
    let mut output: [Vec<f32>; 5] = std::array::from_fn(|_| Vec::with_capacity(n));
    for i in 0..n {
        let p = &mut c.state.f[i * 5..i * 5 + 5];
        if p[4] >= lifetime && active {
            c.state.i[i] += 1;
            let seed = (i as u32).wrapping_mul(747796405)
                .wrapping_add(c.state.i[i] as u32 * 2891336453u32.wrapping_div(4));
            let angle = (seed % 65536) as f32 / 65536.0 * std::f32::consts::TAU;
            p[0] = emitx; p[1] = emity;
            p[2] = angle.cos() * speed; p[3] = angle.sin() * speed;
            p[4] = if c.state.i[i] == 1 { lifetime * i as f32 / n as f32 } else { 0.0 };
        }
        if p[4] < lifetime {
            p[0] += p[2] * dt;
            p[1] += p[3] * dt + 0.5 * gravity * dt * dt;
            p[3] += gravity * dt;
            p[4] += dt;
        }
        let life = (1.0 - p[4] / lifetime).clamp(0.0, 1.0);
        output[0].push((p[0] - 0.5) * 6.0);
        output[1].push((0.5 - p[1]) * 6.0);
        output[2].push(0.0);
        output[3].push(life);
        output[4].push(life);
    }
    ChopData::new(["tx", "ty", "tz", "size", "life"].into_iter()
        .zip(output).map(|(name, samples)| Channel::new(name, samples)).collect(), CONTROL_RATE, false)
}


pub const TYPE_NAME: &str = "particleCHOP";

pub fn spec() -> ChopSpec {
    super::spec_animated(
                TYPE_NAME, "Particle", &[],
                "Stateful 2D particle emitter with velocity, gravity, lifetime and respawn. \
                 tx/ty/tz are instance positions (6 scene units across); size/life fade to zero. \
                 Emitter coordinates are normalized, origin top-left. Connect via geometryCOMP.instancechop; \
                 set sx/sy/sz to size. Active stops births; existing particles finish. \
                 Reset clears the simulation. CPU, at most 4096 particles; dt capped at 0.1s; no collisions or flocking.",
                params, cook,
            )
}
