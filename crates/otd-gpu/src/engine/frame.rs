use otd_core::NodeId;

use crate::record::{self, Recorder};
use super::{DeferredGpuFrame, MovieOutEdge, PendingRecord, TopEngine};

impl TopEngine {
    pub fn set_defer_submit(&mut self, defer: bool) {
        self.defer_submit = defer;
    }

    pub fn flush_deferred_submit(&mut self) -> f64 {
        let started = std::time::Instant::now();
        let pending = std::mem::take(&mut self.deferred_gpu);
        for frame in pending {
            self.ctx.queue.submit([frame.commands]);
            for job in frame.records {
                self.capture(job);
            }
            for t in frame.scratch {
                self.pool.release(t);
            }
        }
        started.elapsed().as_secs_f64() * 1000.0
    }

    pub fn end_frame_immediate(&mut self) {
        let prev = self.defer_submit;
        self.defer_submit = false;
        self.end_frame();
        self.defer_submit = prev;
    }

    pub(super) fn movie_out_edge(&mut self, id: NodeId) -> &mut MovieOutEdge {
        if !self.movie_out_edge.contains_key(id) {
            self.movie_out_edge.insert(id, MovieOutEdge::default());
        }
        self.movie_out_edge.get_mut(id).unwrap()
    }

    pub fn render_stats(&self) -> (u32, u32) {
        self.renderer
            .as_ref()
            .map(|r| (r.draws_this_frame, r.instances_this_frame))
            .unwrap_or((0, 0))
    }

    pub fn context(&self) -> &crate::context::GpuContext {
        &self.ctx
    }

    pub fn begin_frame(&mut self) {
        self.passes_this_frame = 0;
        if let Some(r) = self.renderer.as_mut() {
            r.begin_frame();
        }
        self.encoder = Some(self.ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("otd frame"),
            },
        ));
        self.apply_feedback_snapshots();
    }

    pub fn end_frame(&mut self) {
        for (id, target) in std::mem::take(&mut self.pending_feedback) {
            self.hold_for_next_frame(id, target);
        }
        if let Some(encoder) = self.encoder.take() {
            let commands = encoder.finish();
            if self.defer_submit {
                self.deferred_gpu.push(DeferredGpuFrame {
                    commands,
                    records: std::mem::take(&mut self.pending_records),
                    scratch: std::mem::take(&mut self.scratch),
                });
            } else {
                self.ctx.queue.submit([commands]);
                for job in std::mem::take(&mut self.pending_records) {
                    self.capture(job);
                }
                for t in std::mem::take(&mut self.scratch) {
                    self.pool.release(t);
                }
            }
        } else {
            for job in std::mem::take(&mut self.pending_records) {
                self.capture(job);
            }
            for t in std::mem::take(&mut self.scratch) {
                self.pool.release(t);
            }
        }
    }

    fn capture(&mut self, job: PendingRecord) {
        let id = match &job {
            PendingRecord::Stream(node) | PendingRecord::Still { id: node, .. } => *node,
        };
        let Some(tex) = self.nodes.get(id).and_then(|n| n.output.as_ref()) else {
            return;
        };
        let pixels = match crate::read_pixels_rgba8(&self.ctx, tex) {
            Ok((_, _, px)) => px,
            Err(e) => {
                self.set_node_status(id, Some(format!("could not read the frame back: {e}")));
                return;
            }
        };
        match job {
            PendingRecord::Stream(id) => {
                let Some(recorder) = self.recorders.get(id) else {
                    return;
                };
                if tex.key.width != recorder.width || tex.key.height != recorder.height {
                    return;
                }
                recorder.push(pixels);
            }
            PendingRecord::Still {
                id,
                path,
                image_type,
            } => {
                match record::write_still(
                    &path,
                    tex.key.width,
                    tex.key.height,
                    &pixels,
                    &image_type,
                ) {
                    Ok(()) => {
                        let count = self.movie_out_stills.get(id).copied().unwrap_or(0) + 1;
                        self.movie_out_stills.insert(id, count);
                        if let Some(n) = self.movie_out_seq.get_mut(id) {
                            *n += 1;
                        }
                    }
                    Err(e) => {
                        self.set_node_status(id, Some(e));
                    }
                }
            }
        }
    }

    pub(super) fn movie_out_frames_written(&self, id: NodeId) -> u64 {
        self.recorders
            .get(id)
            .map(Recorder::frames_written)
            .unwrap_or(0)
            + self.movie_out_stills.get(id).copied().unwrap_or(0)
    }
}