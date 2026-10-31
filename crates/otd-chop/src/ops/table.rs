use otd_core::{Connector, Family};

use super::helpers::no_params;
use super::operators::*;
use super::{ChopSpec, IN, NULL, OUT, connector_spec, converter_spec, spec, spec_animated};

pub(super) fn core_specs() -> Vec<ChopSpec> {
    let mut v = vec![
            spec_animated(
                "particleCHOP", "Particle", &[],
                "Stateful 2D particle emitter with velocity, gravity, lifetime and respawn. \
                 tx/ty/tz are instance positions (6 scene units across); size/life fade to zero. \
                 Emitter coordinates are normalized, origin top-left. Connect via geometryCOMP.instancechop; \
                 set sx/sy/sz to size. Active stops births; existing particles finish. \
                 Reset clears the simulation. CPU, at most 4096 particles; dt capped at 0.1s; no collisions or flocking.",
                params_particles, cook_particles,
            ),
            spec(
                "constantCHOP",
                "Constant",
                &[],
                "Fixed values as channels.",
                params_constant,
                cook_constant,
            ),
            spec(
                PARAMETER,
                "Parameter",
                &[],
                "Another operator's parameter values as channels (does not cook that operator).",
                params_parameter,
                cook_parameter,
            ),
            spec_animated(
                INFO,
                "Info",
                &[],
                "Another operator's info channels (TouchDesigner Info CHOP).",
                params_info,
                cook_info,
            ),
            spec_animated(
                "lfoCHOP",
                "LFO",
                &[],
                "A repeating waveform over time.",
                params_lfo,
                cook_lfo,
            ),
            spec_animated(
                "animationCHOP",
                "Animation",
                &[],
                "Keyframed curves over time.",
                params_animation,
                cook_animation,
            ),
            spec_animated(
                "noiseCHOP",
                "Noise",
                &[],
                "Sparse convolution noise over time; channel count comes from Channel Names.",
                params_noise,
                cook_noise,
            ),
            spec(
                "patternCHOP",
                "Pattern",
                &[],
                "A fixed-length waveform buffer.",
                params_pattern,
                cook_pattern,
            ),
            spec(
                "mathCHOP",
                "Math",
                &["a", "b"],
                "Combine and scale channels.",
                params_math,
                cook_math,
            ),
            spec_animated(
                "lagCHOP",
                "Lag",
                &["in"],
                "Smooth a channel, with separate rise and fall times.",
                params_lag,
                cook_lag,
            ),
            spec_animated(
                "filterCHOP",
                "Filter",
                &["in"],
                "Low-pass or moving-average smoothing.",
                params_filter,
                cook_filter,
            ),
            spec(
                "logicCHOP",
                "Logic",
                &["in"],
                "Turn channels into on/off and combine them.",
                params_logic,
                cook_logic,
            ),
            spec_animated(
                "triggerCHOP",
                "Trigger",
                &["in"],
                "An attack/decay/sustain/release envelope per channel.",
                params_trigger,
                cook_trigger,
            ),
            spec_animated(
                "timerCHOP",
                "Timer",
                &[],
                "A running timer with fraction, seconds, cycles and done.",
                params_timer,
                cook_timer,
            ),
            spec_animated(
                "speedCHOP",
                "Speed",
                &["in"],
                "Integrate a channel: velocity becomes position.",
                params_speed,
                cook_speed,
            ),
            spec_animated(
                "countCHOP",
                "Count",
                &["in"],
                "Count threshold crossings.",
                params_count,
                cook_count,
            ),
            spec(
                "selectCHOP",
                "Select",
                &["in"],
                "Pick and rename channels.",
                params_select,
                cook_select,
            ),
            spec(
                "mergeCHOP",
                "Merge",
                &["a", "b"],
                "Put two CHOPs' channels side by side.",
                no_params,
                cook_merge,
            ),
            spec(
                "switchCHOP",
                "Switch",
                &["0", "1"],
                "Choose one of two inputs.",
                params_switch,
                cook_switch,
            ),
            spec(
                NULL,
                "Null",
                &["in"],
                "Pass-through. A stable name to export from.",
                no_params,
                cook_null,
            ),
            spec_animated(
                "delayCHOP",
                "Delay",
                &["in"],
                "Play a channel back later, with optional feedback.",
                params_delay,
                cook_delay,
            ),
            spec_animated(
                "trailCHOP",
                "Trail",
                &["in"],
                "The last few seconds of each channel as one buffer, oldest sample first — \
                 draw a waveform, or lay out one instance per sample.",
                params_trail,
                cook_trail,
            ),
            spec_animated(
                "springCHOP",
                "Spring",
                &["in"],
                "Follow a channel like a mass on a spring: overshoot, wobble, then settle. \
                 Raise damping to stop the wobble; raise the spring constant to follow faster.",
                params_spring,
                cook_spring,
            ),
            spec(
                "fanCHOP",
                "Fan",
                &["in"],
                "Fan out an index channel to one on/off channel per value (chan0..chanN-1), \
                 or fan many channels in to the index of the first one above zero (-1 if none).",
                params_fan,
                cook_fan,
            ),
            spec_animated(
                "expressionCHOP",
                "Expression",
                &["in"],
                "Apply an expression to every sample. `v` is the sample.",
                params_expression,
                cook_expression,
            ),
            spec(
                "limitCHOP",
                "Limit",
                &["in"],
                "Clamp, wrap, fold or quantise a channel.",
                params_limit,
                cook_limit,
            ),
            spec_animated(
                "slopeCHOP",
                "Slope",
                &["in"],
                "The rate of change of a channel.",
                params_slope,
                cook_slope,
            ),
            spec_animated(
                "holdCHOP",
                "Hold",
                &["in", "trigger"],
                "Freeze input 1's value until input 2 fires.",
                params_hold,
                cook_hold,
            ),
            spec(
                "shuffleCHOP",
                "Shuffle",
                &["in"],
                "Rearrange the channel/sample grid — transpose, reverse, sequence.",
                params_shuffle,
                cook_shuffle,
            ),
            spec(
                "renameCHOP",
                "Rename",
                &["in"],
                "Rename channels by pattern.",
                params_rename,
                cook_rename,
            ),
            spec(
                "analyzeCHOP",
                "Analyze",
                &["in"],
                "Reduce each channel to one number.",
                params_analyze,
                cook_analyze,
            ),
            spec(
                "crossCHOP",
                "Cross",
                &["a", "b"],
                "Blend two inputs, with an equal-power option.",
                params_cross,
                cook_cross,
            ),
            spec(
                "resampleCHOP",
                "Resample",
                &["in"],
                "Change how many samples a buffer has, keeping its shape.",
                params_resample,
                cook_resample,
            ),
            spec_animated(
                "panelCHOP",
                "Panel",
                &[],
                "Panel widget values as channels.",
                params_panel,
                cook_panel,
            ),
            spec_animated(
                "clockCHOP",
                "Clock",
                &["target", "reference"],
                "Wall-clock time, which keeps running when the timeline is paused.",
                params_clock,
                cook_clock,
            ),
            spec_animated(
                "beatCHOP",
                "Beat",
                &[],
                "A tempo clock: ramp, pulse, count and bar.",
                params_beat,
                cook_beat,
            ),
            converter_spec(
                "dattochopCHOP",
                "DAT to CHOP",
                Family::Dat,
                "Columns or rows of a table, as channels.",
                params_dat_to,
                cook_dat_to_chop,
            ),
            converter_spec(
                "soptochopCHOP",
                "SOP to CHOP",
                Family::Sop,
                "Point attributes as channels, one sample per point.",
                params_sop_to,
                cook_sop_to_chop,
            ),
            // Animated: a readback has to happen every frame to be worth
            // anything, and the source is a moving picture by assumption.
            {
                let mut s = converter_spec(
                    "blobtrackCHOP", "Blob Track", Family::Top,
                    "Largest connected bright region. Channels: present, count, x, y, area, width, height, vx, vy. \
                     Coordinates and sizes are 0..1, origin top-left; velocity is frame-fractions/second. \
                     CPU readback of last frame; downsample to 160x90 first. \
                     Not person recognition or persistent identity: switching largest regions can jump. \
                     A missing/empty input returns zero channels values.",
                    params_blobtrack, cook_blobtrack,
                );
                s.def.time_dependent = true;
                s
            },
            {
                let mut s = converter_spec(
                    "toptochopCHOP",
                    "TOP to CHOP",
                    Family::Top,
                    "Pixels as channels r g b a. `image` reads every pixel (top row first, at most \
                     128x128) and adds u/v positions, centred, v up — instance a Geometry COMP with \
                     tx=u ty=v tz=g for a camera point cloud. Reads last frame; costs a GPU sync, \
                     so put a Resolution TOP in front.",
                    params_top_to,
                    cook_top_to_chop,
                );
                s.def.time_dependent = true;
                s
            },
            connector_spec(
                IN,
                "In",
                &[],
                "A channel input on this component's node.",
                Connector::In,
            ),
            connector_spec(
                OUT,
                "Out",
                &["in"],
                "This component's channel output.",
                Connector::Out,
            ),
        ];
    v.extend(crate::io::specs());
    v
}
