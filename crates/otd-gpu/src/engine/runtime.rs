use std::collections::HashMap;

use otd_core::cook::resolve_bypass;
use otd_core::{CookContext, CookError, EvalContext, Family, Graph, NodeId};

use crate::ops::WideParams;
use crate::render3d::{self, Renderer};
use crate::scene::Scene;
use crate::texture::TopTexture;

use super::dispatch::{Pass, PipelineRef};
use super::{NodeGpu, PendingRecord, SopPreviewGpu, TopEngine, Uniforms, SOP_PREVIEW_SIZE};

impl TopEngine {
    pub fn output(&self, graph: &Graph, id: NodeId) -> Option<&TopTexture> {
        let real = resolve_bypass(graph, id)?;
        self.nodes.get(real).and_then(|n| n.output.as_ref())
    }
    pub fn shader_error(&self, id: NodeId) -> Option<&str> {
        self.nodes.get(id).and_then(|n| n.shader_error.as_deref())
    }
    pub fn status(&self, id: NodeId) -> Option<&str> {
        self.nodes.get(id).and_then(|n| n.status.as_deref())
    }
    pub fn output_texture_size(&self, id: NodeId) -> Option<(u32, u32)> {
        self.nodes
            .get(id)
            .and_then(|n| n.output.as_ref())
            .map(|t| (t.key.width, t.key.height))
    }
    pub fn sop_preview_texture(&self, id: NodeId) -> Option<&TopTexture> {
        self.sop_previews.get(id).map(|p| &p.output)
    }
    pub fn clear_sop_preview(&mut self, id: NodeId) {
        if let Some(p) = self.sop_previews.remove(id) {
            self.pool.release(p.output);
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn render_sop_preview(
        &mut self,
        id: NodeId,
        geo: &otd_sop::Geometry,
        orbit: &render3d::OrbitCamera,
        wireframe: bool,
        orthographic: bool,
        show_axes: bool,
        geo_stamp: u64,
    ) -> Result<bool, CookError> {
        if geo.is_empty() {
            self.clear_sop_preview(id);
            return Ok(false);
        }
        let key =
            render3d::sop_preview_cache_key(geo_stamp, orbit, wireframe, orthographic, show_axes);
        if self
            .sop_previews
            .get(id)
            .is_some_and(|p| p.cache_key == key)
        {
            return Ok(false);
        }

        let size = SOP_PREVIEW_SIZE;
        let aspect = 1.0;
        let description =
            render3d::describe_sop_preview(geo, aspect, orbit, wireframe, orthographic, show_axes);

        let needs_new = self
            .sop_previews
            .get(id)
            .map(|p| p.output.key.width != size || p.output.key.height != size)
            .unwrap_or(true);
        if needs_new {
            if let Some(old) = self.sop_previews.remove(id) {
                self.pool.release(old.output);
            }
            let output = self.pool.acquire(&self.ctx.device, size, size);
            let depth = render3d::depth_texture(&self.ctx.device, size, size);
            self.sop_previews.insert(
                id,
                SopPreviewGpu {
                    output,
                    depth,
                    cache_key: 0,
                },
            );
        }

        if self.renderer.is_none() {
            self.renderer = Some(Renderer::new(&self.ctx.device));
        }

        let preview = self.sop_previews.get(id).unwrap();
        let target = preview.output.view.clone();
        let depth = preview.depth.clone();
        let dummy = self.dummy.clone();
        let device = self.ctx.device.clone();

        let TopEngine {
            renderer, encoder, ..
        } = self;
        let renderer = renderer.as_mut().unwrap();
        let encoder = encoder.as_mut().ok_or_else(|| {
            CookError::op("SOP viewer", "render outside of begin_frame/end_frame")
        })?;
        renderer.draw(
            &device,
            encoder,
            &target,
            &depth,
            &description,
            &dummy,
            &|_| None,
        );
        self.sop_previews.get_mut(id).unwrap().cache_key = key;
        self.passes_this_frame += 1;
        Ok(true)
    }
    pub fn movie_info(&self, id: NodeId, ch: &mut Vec<(String, f32)>) -> bool {
        if let Some(v) = self.videos.get(id) {
            ch.extend(v.info_channels());
            true
        } else {
            ch.push(("open".into(), 0.0));
            false
        }
    }
    pub fn movie_audio(&self, id: NodeId) -> Option<otd_core::MovieAudio> {
        let slot = self.soundtracks.get(id)?;
        let audio = slot.audio.lock().unwrap_or_else(|e| e.into_inner()).clone();
        Some(audio)
    }
    pub fn frames_recorded(&self, id: NodeId) -> Option<u64> {
        let n = self.movie_out_frames_written(id);
        if n == 0 { None } else { Some(n) }
    }
    pub fn sync_with_graph(&mut self, graph: &Graph) {
        let live = |id: NodeId| graph.contains(id);
        self.nodes.retain(|id, _| live(id));
        self.videos.retain(|id, _| live(id));
        self.soundtracks.retain(|id, _| live(id));
        self.recorders.retain(|id, _| live(id));
        self.movie_ui.retain(|id, _| live(id));
        self.movie_out_edge.retain(|id, _| live(id));
        self.movie_out_seq.retain(|id, _| live(id));
        self.movie_out_stills.retain(|id, _| live(id));
        self.text_keys.retain(|id, _| live(id));
        self.sop_previews.retain(|id, _| live(id));
        for (id, node) in graph.iter() {
            if node.family == Family::Top && !self.nodes.contains_key(id) {
                self.nodes.insert(id, NodeGpu::default());
            }
        }
    }
    pub fn forget(&mut self, id: NodeId) {
        self.release_video(id);
        self.movie_ui.remove(id);
        self.movie_out_edge.remove(id);
        self.movie_out_seq.remove(id);
        self.movie_out_stills.remove(id);
        // Deleting a recording node finalises its file rather than truncating
        // it: the drop closes the pipe and waits for ffmpeg's trailer.
        self.recorders.remove(id);
        self.pending_records.retain(|p| match p {
            PendingRecord::Stream(node) => *node != id,
            PendingRecord::Still { id: node, .. } => *node != id,
        });
        self.text_keys.remove(id);
        if let Some(mut n) = self.nodes.remove(id) {
            if let Some(t) = n.output.take() {
                self.pool.release(t);
            }
        }
        if let Some(p) = self.sop_previews.remove(id) {
            self.pool.release(p.output);
        }
        self.chop_pixel_cache.remove(&id);
        self.chop_pixel_wanted.remove(&id);
        self.chop_pixel_pending.remove(&id);
    }
    pub fn want_chop_pixel_readback(&mut self, id: NodeId) {
        self.chop_pixel_wanted.insert(id);
    }
    pub fn chop_pixel_cache(&self, id: NodeId) -> Option<(u32, u32, Vec<f32>)> {
        self.chop_pixel_cache.get(&id).cloned()
    }
    pub fn poll_chop_pixel_readbacks(&mut self) {
        while !self.chop_pixel_pending.is_empty() {
            let ids: Vec<NodeId> = self.chop_pixel_pending.keys().copied().collect();
            for id in ids {
                let Some(mut pending) = self.chop_pixel_pending.remove(&id) else {
                    continue;
                };
                match crate::poll_pixel_readback(&self.ctx, &mut pending) {
                    Ok(Some((w, h, rgba))) => {
                        self.chop_pixel_cache.insert(id, (w, h, rgba));
                    }
                    Ok(None) => {
                        self.chop_pixel_pending.insert(id, pending);
                    }
                    Err(_) => {}
                }
            }
            if !self.chop_pixel_pending.is_empty() {
                let _ = self
                    .ctx
                    .device
                    .poll(wgpu::PollType::wait_indefinitely());
            }
        }
    }
    pub fn schedule_chop_pixel_readbacks(&mut self, graph: &Graph) {
        self.poll_chop_pixel_readbacks();
        let wanted = std::mem::take(&mut self.chop_pixel_wanted);
        for id in wanted {
            if self.chop_pixel_pending.contains_key(&id) {
                continue;
            }
            let Some(tex) = self.output(graph, id) else {
                continue;
            };
            if let Ok(pending) = crate::start_read_pixels_rgba_f32(&self.ctx, tex) {
                self.chop_pixel_pending.insert(id, pending);
            }
        }
    }
    pub fn reset(&mut self) {
        let ids: Vec<NodeId> = self.nodes.keys().collect();
        for id in ids {
            self.forget(id);
        }
        let sop_ids: Vec<NodeId> = self.sop_previews.keys().collect();
        for id in sop_ids {
            self.clear_sop_preview(id);
        }
        self.pool.clear();
    }
    pub fn shutdown(&mut self) {
        let recorder_ids: Vec<NodeId> = self.recorders.keys().collect();
        for id in recorder_ids {
            if let Some(mut rec) = self.recorders.remove(id) {
                rec.abort();
            }
        }
        self.videos.clear();
        self.pending_records.clear();
    }
    pub fn pooled_bytes(&self) -> u64 {
        self.pool.pooled_bytes()
    }
    pub fn textures_created(&self) -> u64 {
        self.pool.created
    }
    pub fn resident_bytes(&self) -> u64 {
        self.nodes
            .values()
            .filter_map(|n| n.output.as_ref())
            .map(|t| t.key.width as u64 * t.key.height as u64 * 8)
            .sum()
    }
    pub fn presented(&self, graph: &Graph, id: NodeId) -> Option<&TopTexture> {
        self.output(graph, id)
    }
    pub(super) fn ensure_node_mut(&mut self, id: NodeId) -> Result<&mut NodeGpu, CookError> {
        if !self.nodes.contains_key(id) {
            self.nodes.insert(id, NodeGpu::default());
        }
        self.nodes.get_mut(id).ok_or(CookError::NoSuchNode)
    }
    pub(super) fn set_node_status(&mut self, id: NodeId, status: Option<String>) {
        if let Ok(n) = self.ensure_node_mut(id) {
            n.status = status;
        }
    }
    pub(super) fn ensure_output(&mut self, id: NodeId, width: u32, height: u32) {
        if !self.nodes.contains_key(id) {
            self.nodes.insert(id, NodeGpu::default());
        }
        let matches = self
            .nodes
            .get(id)
            .and_then(|n| n.output.as_ref())
            .map(|t| t.key.width == width && t.key.height == height)
            .unwrap_or(false);
        if matches {
            return;
        }
        let old = self.nodes.get_mut(id).and_then(|n| n.output.take());
        let fresh = self.pool.acquire(&self.ctx.device, width, height);
        if let Some(n) = self.nodes.get_mut(id) {
            n.output = Some(fresh);
        }
        if let Some(old) = old {
            self.pool.release(old);
        }
    }
    pub(super) fn write_uniform(&mut self, id: NodeId, slot: usize, data: Uniforms) -> wgpu::Buffer {
        let _ = self.ensure_node_mut(id);
        while self.nodes[id].uniforms.len() <= slot {
            let buf = self.ctx.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("otd top uniforms"),
                size: std::mem::size_of::<Uniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.nodes[id].uniforms.push(buf);
        }
        let buf = self.nodes[id].uniforms[slot].clone();
        self.ctx
            .queue
            .write_buffer(&buf, 0, bytemuck::bytes_of(&data));
        buf
    }
    pub(super) fn run_pass(&mut self, pass: Pass<'_>) -> Result<(), CookError> {
        // Pipeline lookup borrows self mutably; clone the handle out so the
        // bind group and the encoder can be touched afterwards.
        let pipeline = match pass.pipeline {
            PipelineRef::Builtin(name) => self.builtin_pipeline(name)?.clone(),
            PipelineRef::Key(key) => self
                .pipelines
                .get(key)
                .ok_or_else(|| CookError::op(pass.label, "shader is not compiled"))?
                .clone(),
        };
        let sampler = if pass.nearest {
            self.sampler_nearest.clone()
        } else {
            self.sampler_linear.clone()
        };
        let table = self
            .table_override
            .clone()
            .unwrap_or_else(|| self.dummy_table.clone());
        let bind = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(pass.label),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: pass.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(pass.in0),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(pass.in1),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&table),
                    },
                ],
            });

        let encoder = self
            .encoder
            .as_mut()
            .ok_or_else(|| CookError::op(pass.label, "cook outside of begin_frame/end_frame"))?;
        {
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(pass.label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: pass.target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render.set_pipeline(&pipeline);
            render.set_bind_group(0, &bind, &[]);
            render.draw(0..3, 0..1);
        }
        self.passes_this_frame += 1;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn cook_render(
        &mut self,
        graph: &Graph,
        id: NodeId,
        _ctx: &CookContext,
        eval: &EvalContext,
        scene: Option<&dyn Scene>,
        width: u32,
        height: u32,
    ) -> Result<(), CookError> {
        let Some(scene) = scene else {
            // A pure-TOP host has no geometry to give us; leave the last
            // frame rather than clearing to black.
            return Ok(());
        };
        let node = graph.get(id).ok_or(CookError::NoSuchNode)?;
        let description = render3d::describe(
            graph,
            node,
            eval,
            scene,
            width as f32 / height.max(1) as f32,
        );

        // A depth buffer matched to the colour target, rebuilt only on resize.
        if !self.nodes.contains_key(id) {
            self.nodes.insert(id, NodeGpu::default());
        }
        let needs_depth = self
            .nodes
            .get(id)
            .map(|n| {
                n.depth
                    .as_ref()
                    .map(|(w, h, _)| *w != width || *h != height)
                    .unwrap_or(true)
            })
            .unwrap_or(true);
        if needs_depth {
            let view = render3d::depth_texture(&self.ctx.device, width, height);
            if let Some(n) = self.nodes.get_mut(id) {
                n.depth = Some((width, height, view));
            }
        }

        let target = self.nodes[id].output.as_ref().unwrap().view.clone();
        let depth = self.nodes[id].depth.as_ref().unwrap().2.clone();
        if self.renderer.is_none() {
            self.renderer = Some(Renderer::new(&self.ctx.device));
        }

        // Colour maps are looked up before the borrow of the renderer.
        let maps: HashMap<NodeId, wgpu::TextureView> = description
            .items
            .iter()
            .filter_map(|i| i.color_map)
            .filter_map(|m| self.output(graph, m).map(|t| (m, t.view.clone())))
            .collect();
        let dummy = self.dummy.clone();
        let device = self.ctx.device.clone();

        let TopEngine {
            renderer, encoder, ..
        } = self;
        let renderer = renderer.as_mut().unwrap();
        let encoder = encoder
            .as_mut()
            .ok_or_else(|| CookError::op("renderTOP", "cook outside of a frame"))?;
        renderer.draw(
            &device,
            encoder,
            &target,
            &depth,
            &description,
            &dummy,
            &|id| maps.get(&id).cloned(),
        );
        self.passes_this_frame += 1;
        Ok(())
    }
    pub(super) fn input_view(&self, graph: &Graph, node_id: NodeId, index: usize) -> wgpu::TextureView {
        graph
            .resolve_input_source(node_id, index)
            .and_then(|src| self.output(graph, src))
            .map(|t| t.view.clone())
            .unwrap_or_else(|| self.dummy.clone())
    }
    pub(super) fn input_size(&self, graph: &Graph, node_id: NodeId, index: usize) -> Option<(u32, u32)> {
        let src = graph.resolve_input_source(node_id, index)?;
        let t = self.output(graph, src)?;
        Some((t.key.width, t.key.height))
    }
    pub(super) fn uniforms(&self, width: u32, height: u32, ctx: &CookContext, params: WideParams) -> Uniforms {
        Uniforms {
            res: [
                width as f32,
                height as f32,
                1.0 / width as f32,
                1.0 / height as f32,
            ],
            time: [
                ctx.abs_time as f32,
                ctx.time as f32,
                ctx.frame as f32,
                ctx.fps as f32,
            ],
            params,
        }
    }
    pub(super) fn attach_soundtrack(&mut self, id: NodeId, path: &str) {
        if self.soundtracks.get(id).is_some_and(|s| s.path == path) {
            return;
        }
        self.soundtracks
            .insert(id, crate::video::spawn_movie_audio(path.to_string()));
    }

    pub(super) fn release_video(&mut self, id: NodeId) {
        self.videos.remove(id);
        self.soundtracks.remove(id);
    }

}
