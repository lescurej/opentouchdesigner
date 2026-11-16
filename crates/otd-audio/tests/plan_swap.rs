use otd_audio::{plan_channel, AudioExecutor, AudioKernel, AudioPlan, BlockCtx, PlanStep};

struct Constant(f32);

impl AudioKernel for Constant {
    fn prepare(&mut self, _: f64, _: usize) {}
    fn process(&mut self, _: &BlockCtx, _: &[&[f32]], output: &mut [f32]) {
        output.fill(self.0);
    }
}

fn constant_plan(value: f32, rate: f64) -> Box<AudioPlan> {
    let mut plan = AudioPlan::new(
        64,
        vec![1],
        vec![Box::new(Constant(value))],
        vec![PlanStep {
            kernel: 0,
            inputs: Vec::new(),
            output: 0,
        }],
    );
    plan.sink_slot = Some(0);
    plan.prepare(rate);
    Box::new(plan)
}

#[test]
fn published_plan_replaces_the_live_one_and_the_old_one_comes_back() {
    let (mut tx, rx) = plan_channel(2);
    let mut ex = AudioExecutor::new(constant_plan(0.25, 48_000.0), Some(rx), None);
    let mut out = vec![0.0; 64];
    ex.render(&mut out, 1);
    assert!(out.iter().all(|s| *s == 0.25));

    tx.publish(constant_plan(0.75, 44_100.0)).unwrap();
    ex.render(&mut out, 1);
    assert!(out.iter().all(|s| *s == 0.75));
    assert_eq!(ex.plan().sample_rate, 44_100.0);
    assert_eq!(tx.collect_retired(), 1, "old plan is freed on the control thread");
}

#[test]
fn unprepared_plans_are_rejected_without_touching_the_live_plan() {
    let (mut tx, rx) = plan_channel(2);
    let mut ex = AudioExecutor::new(constant_plan(0.5, 48_000.0), Some(rx), None);
    let raw = AudioPlan::new(64, vec![1], vec![Box::new(Constant(1.0))], Vec::new());
    tx.publish(Box::new(raw)).unwrap();
    let mut out = vec![0.0; 64];
    ex.render(&mut out, 1);
    assert!(out.iter().all(|s| *s == 0.5));
    assert_eq!(tx.collect_retired(), 1);
}

#[test]
fn render_splits_large_callbacks_into_plan_blocks_and_maps_channels() {
    let mut ex = AudioExecutor::new(constant_plan(0.5, 48_000.0), None, None);
    let mut out = vec![0.0; 1000 * 2];
    ex.render(&mut out, 2);
    assert!(out.iter().all(|s| *s == 0.5));
}
