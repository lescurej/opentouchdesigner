use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

use otd_audio::{
    plan_channel, tap_channel, AudioExecutor, AudioKernel, AudioPlan, BlockCtx, ControlPort,
    PlanStep,
};

struct CountingAlloc;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static EVENTS: Cell<usize> = const { Cell::new(0) };
}

fn record() {
    let _ = COUNTING.try_with(|on| {
        if on.get() {
            let _ = EVENTS.try_with(|e| e.set(e.get() + 1));
        }
    });
}

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record();
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record();
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

fn allocations_during(f: impl FnOnce()) -> usize {
    EVENTS.with(|e| e.set(0));
    COUNTING.with(|on| on.set(true));
    f();
    COUNTING.with(|on| on.set(false));
    EVENTS.with(|e| e.get())
}

struct Sine {
    phase: f32,
    step: f32,
}

impl AudioKernel for Sine {
    fn prepare(&mut self, sample_rate: f64, _: usize) {
        self.step = 440.0 / sample_rate as f32;
    }
    fn process(&mut self, _: &BlockCtx, _: &[&[f32]], output: &mut [f32]) {
        for s in output.iter_mut() {
            *s = (self.phase * std::f32::consts::TAU).sin();
            self.phase = (self.phase + self.step).fract();
        }
    }
}

struct MixToStereo;

impl AudioKernel for MixToStereo {
    fn prepare(&mut self, _: f64, _: usize) {}
    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]) {
        for fi in 0..ctx.block_len {
            for (c, input) in inputs.iter().take(2).enumerate() {
                output[fi * 2 + c] = input.get(fi).copied().unwrap_or(0.0);
            }
        }
    }
}

struct Gain(Arc<ControlPort>);

impl AudioKernel for Gain {
    fn prepare(&mut self, _: f64, _: usize) {}
    fn process(&mut self, ctx: &BlockCtx, inputs: &[&[f32]], output: &mut [f32]) {
        let ramp = self.0.ramp(ctx.block_len);
        let Some(input) = inputs.first() else {
            return;
        };
        for (i, s) in output.iter_mut().enumerate() {
            *s = input.get(i).copied().unwrap_or(0.0) * ramp.at(i / 2);
        }
    }
}

fn stereo_plan(volume: Arc<ControlPort>) -> Box<AudioPlan> {
    let mut plan = AudioPlan::new(
        256,
        vec![1, 1, 2, 2],
        vec![
            Box::new(Sine { phase: 0.0, step: 0.0 }),
            Box::new(Sine { phase: 0.5, step: 0.0 }),
            Box::new(MixToStereo),
            Box::new(Gain(volume)),
        ],
        vec![
            PlanStep { kernel: 0, inputs: vec![], output: 0 },
            PlanStep { kernel: 1, inputs: vec![], output: 1 },
            PlanStep { kernel: 2, inputs: vec![0, 1], output: 2 },
            PlanStep { kernel: 3, inputs: vec![2], output: 3 },
        ],
    );
    plan.sink_slot = Some(3);
    plan.tap_slot = Some(2);
    plan.prepare(48_000.0);
    Box::new(plan)
}

#[test]
fn render_swap_and_tap_never_touch_the_allocator() {
    let volume = Arc::new(ControlPort::new(1.0));
    let (mut tx, rx) = plan_channel(4);
    let (tap_tx, mut tap_rx) = tap_channel(512 * 2 * 64);
    let mut ex = AudioExecutor::new(stereo_plan(volume.clone()), Some(rx), Some(tap_tx));
    let mut out = vec![0.0f32; 512 * 2];
    let next = stereo_plan(volume.clone());

    let steady = allocations_during(|| {
        for i in 0..32 {
            volume.set(if i % 2 == 0 { 0.2 } else { 0.9 });
            ex.render(&mut out, 2);
        }
    });
    assert_eq!(steady, 0, "steady-state render allocated");

    tx.publish(next).unwrap();
    let swapped = allocations_during(|| ex.render(&mut out, 2));
    assert_eq!(swapped, 0, "plan swap allocated or freed on the audio thread");
    assert_eq!(tx.collect_retired(), 1);

    let mut tapped = Vec::new();
    tap_rx.drain_into(&mut tapped);
    assert_eq!(tapped.len(), 33 * 512 * 2);
    assert!(out.iter().any(|s| s.abs() > 0.01));
}
