use crate::plan::AudioPlan;
use crate::swap::PlanReceiver;
use crate::tap::TapWriter;

/// Runs plans block by block. Owned by exactly one thread; `render` never
/// allocates, locks or frees.
pub struct AudioExecutor {
    plan: Box<AudioPlan>,
    plans: Option<PlanReceiver>,
    tap: Option<TapWriter>,
    pending_retire: Option<Box<AudioPlan>>,
    tap_drops: u64,
}

impl AudioExecutor {
    pub fn new(mut plan: Box<AudioPlan>, plans: Option<PlanReceiver>, tap: Option<TapWriter>) -> Self {
        if !plan.is_prepared() {
            plan.prepare(48_000.0);
        }
        Self {
            plan,
            plans,
            tap,
            pending_retire: None,
            tap_drops: 0,
        }
    }

    pub fn plan(&self) -> &AudioPlan {
        &self.plan
    }

    pub fn tap_drops(&self) -> u64 {
        self.tap_drops
    }

    fn swap_plan(&mut self) {
        let Some(rx) = self.plans.as_mut() else {
            return;
        };
        if let Some(old) = self.pending_retire.take()
            && let Err(old) = rx.retire(old)
        {
            self.pending_retire = Some(old);
            return;
        }
        while let Some(incoming) = rx.take() {
            if !incoming.is_prepared() {
                if let Err(bad) = rx.retire(incoming) {
                    self.pending_retire = Some(bad);
                    return;
                }
                continue;
            }
            let old = std::mem::replace(&mut self.plan, incoming);
            if let Err(old) = rx.retire(old) {
                self.pending_retire = Some(old);
                return;
            }
        }
    }

    /// Fills `out` (interleaved, `out_channels` wide) with the sink slot.
    pub fn render(&mut self, out: &mut [f32], out_channels: usize) {
        self.swap_plan();
        let out_ch = out_channels.max(1);
        let total = out.len() / out_ch;
        let block = self.plan.max_block;
        let mut done = 0;
        while done < total {
            let frames = (total - done).min(block);
            let dst = &mut out[done * out_ch..(done + frames) * out_ch];
            self.render_block(dst, out_ch, frames);
            done += frames;
        }
    }

    fn render_block(&mut self, dst: &mut [f32], out_ch: usize, frames: usize) {
        if self.plan.kernels.is_empty() {
            dst.fill(0.0);
            return;
        }
        self.plan.run_block(frames);
        match self.plan.sink_slot {
            Some(slot) => {
                let ch = self.plan.slot_channels[slot].max(1);
                let src = self.plan.slot(slot, frames);
                for fi in 0..frames {
                    for c in 0..out_ch {
                        dst[fi * out_ch + c] = src.get(fi * ch + c.min(ch - 1)).copied().unwrap_or(0.0);
                    }
                }
            }
            None => dst.fill(0.0),
        }
        if let (Some(slot), Some(tap)) = (self.plan.tap_slot, self.tap.as_mut())
            && !tap.write(self.plan.slot(slot, frames))
        {
            self.tap_drops += 1;
        }
    }
}
