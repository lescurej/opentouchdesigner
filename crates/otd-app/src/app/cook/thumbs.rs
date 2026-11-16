use super::{VIEWER_THUMB_MAX, VIEWER_THUMB_MAX_PLAYING};
use egui::TextureId;
use otd_core::NodeId;
use super::super::NativeTopView;

impl super::super::OtdApp {
    pub fn thumbnail(&mut self, id: NodeId) -> Option<(TextureId, [u32; 2])> {
        let (w, h) = self.snapshot_top_meta(id).or_else(|| {
            self.engine
                .top_output_meta(&self.graph, id)
                .map(|(w, h, _)| (w, h))
        })?;
        let version = self.snapshot_output_version(id);
        let size = [w, h];
        self.thumb_sizes.insert(id, size);
        let (tid, needs_sync) = match self.thumbs.get(&id) {
            Some(thumb) if thumb.version == version => (thumb.egui_id, false),
            Some(thumb) => (thumb.egui_id, true),
            None => {
                if self.wants_live_thumbnail(id) {
                    self.queue_texture_sync(id);
                }
                return None;
            }
        };
        if needs_sync {
            if self.wants_live_thumbnail(id) {
                self.queue_texture_sync(id);
            } else if let Some(thumb) = self.thumbs.get(&id) {
                return Some((thumb.egui_id, size));
            }
        }
        Some((tid, size))
    }

    pub(crate) fn queue_texture_sync(&mut self, id: NodeId) {
        if !self.pending_texture_sync.contains(&id) {
            self.pending_texture_sync.push(id);
        }
    }

    pub(crate) fn sync_playing_display_blits(&mut self, ctx: &egui::Context) {
        if !self.transport.playing {
            return;
        }
        if self.frame_perf_probe.did_cook && self.background_engine.is_paused_for_ui() {
            return;
        }
        let queue: Vec<NodeId> = self
            .canvas.visible
            .iter()
            .copied()
            .filter(|id| self.wants_live_thumbnail(*id))
            .collect();
        for id in queue {
            self.queue_texture_sync(id);
        }
        self.run_thumb_blit_batch(ctx, VIEWER_THUMB_MAX_PLAYING, true);
    }

    pub(crate) fn sync_pending_textures(&mut self, ctx: &egui::Context) {
        if self.transport.playing {
            return;
        }
        self.run_thumb_blit_batch(ctx, VIEWER_THUMB_MAX, true);
    }

    fn run_thumb_blit_batch(&mut self, ctx: &egui::Context, thumb_cap: u32, update_probe: bool) {
        let sync_started = std::time::Instant::now();
        let batch = self.take_texture_sync_batch();
        let batch_len = batch.len() as u32;
        if batch.is_empty() {
            if update_probe {
                self.frame_perf_probe.sync_ms = sync_started.elapsed().as_secs_f64() * 1000.0;
                self.frame_perf_probe.sync_blits = 0;
            }
            return;
        }

        let versions: Vec<(NodeId, u64)> = batch
            .iter()
            .map(|id| (*id, self.snapshot_output_version(*id)))
            .collect();
        for id in &batch {
            if !self.prepare_thumb_rgba_target(ctx, *id, thumb_cap) {
                self.free_thumbnail(*id);
            }
        }

        let time = self.time;
        let blit_targets: Vec<(NodeId, wgpu::TextureView)> = batch
            .iter()
            .filter_map(|id| self.thumbs.get(id).map(|t| (*id, t.view.clone())))
            .collect();
        let mut rt = self.engine.runtime.lock().expect("engine runtime lock");
        rt.engines.flush_deferred_gpu_submit();
        rt.engines.top.begin_frame();
        let mut ok_ids = Vec::new();
        let graph = &self.graph;
        for (id, dst) in &blit_targets {
            let blit_ok = match rt.engines.top.output(graph, *id).cloned() {
                None => false,
                Some(src) => rt
                    .engines
                    .top
                    .blit_to_rgba8_unorm(graph, &src, dst, &time)
                    .is_ok(),
            };
            if blit_ok {
                ok_ids.push(*id);
            }
        }
        rt.engines.finish_thumb_blit_frame();
        drop(rt);

        let ok: std::collections::HashSet<_> = ok_ids.into_iter().collect();
        for id in batch {
            if !ok.contains(&id) {
                self.free_thumbnail(id);
                continue;
            }
            let version = versions
                .iter()
                .find(|(nid, _)| *nid == id)
                .map(|(_, v)| *v)
                .unwrap_or(0);
            if let Some(thumb) = self.thumbs.get_mut(&id) {
                thumb.version = version;
            }
        }
        if update_probe {
            self.frame_perf_probe.sync_ms = sync_started.elapsed().as_secs_f64() * 1000.0;
            self.frame_perf_probe.sync_blits = batch_len;
        }
    }

    fn prepare_thumb_rgba_target(
        &mut self,
        _ctx: &egui::Context,
        id: NodeId,
        thumb_cap: u32,
    ) -> bool {
        let (src_w, src_h) = match self.snapshot_top_meta(id).or_else(|| {
            self.engine
                .top_output_meta(&self.graph, id)
                .map(|(w, h, _)| (w, h))
        }) {
            Some(dim) => dim,
            None => return false,
        };
        let longest = src_w.max(src_h).max(1);
        let (w, h) = if longest > thumb_cap {
            let s = thumb_cap as f32 / longest as f32;
            (
                (src_w as f32 * s).round().max(1.0) as u32,
                (src_h as f32 * s).round().max(1.0) as u32,
            )
        } else {
            (src_w, src_h)
        };
        self.thumb_sizes.insert(id, [src_w, src_h]);

        let need_alloc = self
            .thumbs
            .get(&id)
            .map(|t| t.texture.width() != w || t.texture.height() != h)
            .unwrap_or(true);
        if !need_alloc {
            return true;
        }

        let device = self.render_state.device.clone();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("otd viewer rgba8"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let egui_id = if let Some(old) = self.thumbs.get(&id) {
            self.render_state
                .renderer
                .write()
                .update_egui_texture_from_wgpu_texture(
                    &device,
                    &view,
                    wgpu::FilterMode::Linear,
                    old.egui_id,
                );
            old.egui_id
        } else {
            self.render_state.renderer.write().register_native_texture(
                &device,
                &view,
                wgpu::FilterMode::Linear,
            )
        };
        self.thumbs.insert(
            id,
            NativeTopView {
                texture,
                view,
                egui_id,
                version: 0,
            },
        );
        true
    }
}
