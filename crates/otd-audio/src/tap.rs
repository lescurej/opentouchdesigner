use rtrb::{Consumer, Producer, RingBuffer};

pub struct TapWriter {
    producer: Producer<f32>,
}

pub struct TapReader {
    consumer: Consumer<f32>,
}

pub fn tap_channel(capacity_samples: usize) -> (TapWriter, TapReader) {
    let (producer, consumer) = RingBuffer::new(capacity_samples.max(1));
    (TapWriter { producer }, TapReader { consumer })
}

impl TapWriter {
    /// Writes a whole block or nothing, so the reader never sees a torn frame.
    pub(crate) fn write(&mut self, interleaved: &[f32]) -> bool {
        if self.producer.slots() < interleaved.len() {
            return false;
        }
        let _ = self.producer.push_partial_slice(interleaved);
        true
    }
}

impl TapReader {
    pub fn drain_into(&mut self, out: &mut Vec<f32>) -> usize {
        let n = self.consumer.slots();
        let start = out.len();
        out.resize(start + n, 0.0);
        let (filled, _) = self.consumer.pop_partial_slice(&mut out[start..]);
        let got = filled.len();
        out.truncate(start + got);
        got
    }
}
