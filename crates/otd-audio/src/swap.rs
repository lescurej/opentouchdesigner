use rtrb::{Consumer, Producer, RingBuffer};

use crate::plan::AudioPlan;

/// Control-thread end: publishes prepared plans and frees the retired ones,
/// so no plan is ever dropped on the audio worker.
pub struct PlanSender {
    to_audio: Producer<Box<AudioPlan>>,
    retired: Consumer<Box<AudioPlan>>,
}

/// Audio-worker end.
pub struct PlanReceiver {
    incoming: Consumer<Box<AudioPlan>>,
    retired: Producer<Box<AudioPlan>>,
}

pub fn plan_channel(capacity: usize) -> (PlanSender, PlanReceiver) {
    let capacity = capacity.max(1);
    let (to_audio, incoming) = RingBuffer::new(capacity);
    let (retired_tx, retired) = RingBuffer::new(capacity + 1);
    (
        PlanSender { to_audio, retired },
        PlanReceiver {
            incoming,
            retired: retired_tx,
        },
    )
}

impl PlanSender {
    pub fn publish(&mut self, plan: Box<AudioPlan>) -> Result<(), Box<AudioPlan>> {
        self.collect_retired();
        self.to_audio.push(plan).map_err(|rtrb::PushError::Full(p)| p)
    }

    pub fn collect_retired(&mut self) -> usize {
        let mut freed = 0;
        while self.retired.pop().is_ok() {
            freed += 1;
        }
        freed
    }
}

impl PlanReceiver {
    pub(crate) fn take(&mut self) -> Option<Box<AudioPlan>> {
        self.incoming.pop().ok()
    }

    pub(crate) fn retire(&mut self, plan: Box<AudioPlan>) -> Result<(), Box<AudioPlan>> {
        self.retired.push(plan).map_err(|rtrb::PushError::Full(p)| p)
    }
}
