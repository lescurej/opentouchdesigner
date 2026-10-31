use super::*;

impl OtdApp {
    fn cook_roots(&self) -> Vec<NodeId> {
        let perform_ui = self.perform_ui();
        let mut roots = ui::cook_roots(&self.graph, perform_ui, &self.visible, self.current);
        crate::window_comp::append_cook_roots(&self.graph, &mut roots);
        if let Some(top) = crate::window_comp::perform_display_top(
            &self.graph,
            self.main_perform_window,
            perform_ui,
        ) {
            if !roots.contains(&top) {
                roots.push(top);
            }
        }
        if let Some(comp) = perform_ui {
            crate::canvas::comp_display::append_perform_panel_cook_roots(
                &self.graph,
                comp,
                &mut roots,
            );
        }
        crate::container_view::append_cook_roots(&self.graph, &self.open_container_views, &mut roots);
        roots
    }

    pub(crate) fn cook_frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last_instant).as_secs_f64().min(0.25);
        self.last_instant = now;
        self.smoothed_fps = self.smoothed_fps * 0.9 + (1.0 / dt.max(1e-6)) * 0.1;

        if self.playing && !self.scrubbing {
            self.time.advance(dt);
            let (start, end) = self.loop_range;
            if self.looping && end > start && self.time.time >= end {
                // Wrap by the loop length rather than snapping to the start:
                // the leftover carries into the next pass, so a loop shorter
                // than a frame interval does not drift or stall.
                let length = end - start;
                self.time.time = start + (self.time.time - start).rem_euclid(length);
            }
        }
        self.scrubbing = false;

        // Clones follow their master. An unchanged master costs one subtree
        // walk, so this is cheap enough to do every frame and means the
        // editor never shows a stale copy.
        // Replicators first — they may create clones for syncing to fill.
        let replicated = otd_engine::replicator::sync(&mut self.graph, &self.engines.dats);
        let synced = self.graph.sync_clones(&self.registry);
        if replicated > 0 {
            self.status = format!("replicated {replicated} node(s)");
        } else if synced > 0 {
            self.status = format!("synced {synced} clone(s)");
        }

        let roots = self.cook_roots();
        self.engines.set_input_state(self.input_state.clone());
        self.engines.begin_frame();
        let result =
            self.cook
                .cook_frame(&self.graph, &roots, &self.time.clone(), &mut self.engines);
        self.engines.end_frame();

        self.cook_error = result.err().map(|e| e.to_string());
        let frame = self.time.frame;
        if let Some(err) = &self.cook_error {
            if self.textport.last_cook_error.as_deref() != Some(err.as_str()) {
                self.textport.error(frame, err.clone());
                self.textport.last_cook_error = Some(err.clone());
            }
        } else {
            self.textport.last_cook_error = None;
        }

        // Between frames, where the graph is held mutably, is the only place a
        // callback's parameter change can land. See `otd_core::edit`.
        let edits = self.engines.take_edits();
        if !edits.is_empty() {
            let (applied, problems) = otd_core::edit::apply(&mut self.graph, &edits);
            for p in &problems {
                self.textport.warn(frame, p);
            }
            if let Some(first) = problems.first() {
                self.status = format!("callback: {first}");
            } else if applied > 0 {
                self.status = format!("callback set {applied} parameter(s)");
                self.textport
                    .info(frame, format!("callback applied {applied} parameter edit(s)"));
            }
        }
        for line in self.engines.take_script_log() {
            self.textport.info(frame, line);
        }

        let ms = self.cook.stats.total_cook_us as f64 / 1000.0;
        self.smoothed_cook_ms = self.smoothed_cook_ms * 0.9 + ms * 0.1;
    }

    /// The egui texture handle for a node's current output, registering or
    /// refreshing it only when the underlying texture object changed.
    pub fn set_perform_ui(&mut self, id: NodeId) {
        if !self.graph.contains(id) {
            return;
        }
        if self.graph.node(id).family != otd_core::Family::Comp {
            self.status = "Perform UI must be a COMP".into();
            return;
        }
        self.edit("perform UI");
        self.history.end_gesture();
        self.main_perform_ui_path = self.graph.path(id);
        self.viewer = Some(id);
        self.queue_enter_network(id);
        self.select_only(id);
    }

    pub fn toggle_perform_ui(&mut self, id: NodeId) {
        if self.perform_ui() == Some(id) {
            self.edit("perform UI");
            self.history.end_gesture();
            self.main_perform_ui_path.clear();
            self.viewer = None;
        } else {
            self.set_perform_ui(id);
        }
    }

    pub fn perform_output_thumbnail(&mut self) -> Option<(TextureId, [u32; 2])> {
        let out = crate::window_comp::perform_display_top(
            &self.graph,
            self.main_perform_window,
            self.perform_ui(),
        )?;
        self.thumbnail(out)
    }

    pub fn thumbnail(&mut self, id: NodeId) -> Option<(TextureId, [u32; 2])> {
        let tex = self.engines.top.output(&self.graph, id)?.clone();
        let size = [tex.key.width, tex.key.height];
        let entry = self.thumbs.get(&id).copied();
        match entry {
            Some((tid, seen)) if seen == tex.generation => Some((tid, size)),
            Some((tid, _)) => {
                self.queue_texture_sync(id);
                Some((tid, size))
            }
            None => {
                self.queue_texture_sync(id);
                None
            }
        }
    }

    fn queue_texture_sync(&mut self, id: NodeId) {
        if !self.pending_texture_sync.contains(&id) {
            self.pending_texture_sync.push(id);
        }
    }

    pub(crate) fn sync_pending_textures(&mut self) {
        let pending = std::mem::take(&mut self.pending_texture_sync);
        for id in pending {
            self.sync_texture_for_node(id);
        }
    }

    fn sync_texture_for_node(&mut self, id: NodeId) {
        let Some(tex) = self.engines.top.output(&self.graph, id).cloned() else {
            self.free_thumbnail(id);
            return;
        };
        let entry = self.thumbs.get(&id).copied();
        match entry {
            Some((_tid, seen)) if seen == tex.generation => {}
            Some((tid, _)) => {
                self.render_state
                    .renderer
                    .write()
                    .update_egui_texture_from_wgpu_texture(
                        &self.render_state.device,
                        &tex.view,
                        wgpu::FilterMode::Linear,
                        tid,
                    );
                self.thumbs.insert(id, (tid, tex.generation));
            }
            None => {
                let tid = self.render_state.renderer.write().register_native_texture(
                    &self.render_state.device,
                    &tex.view,
                    wgpu::FilterMode::Linear,
                );
                self.thumbs.insert(id, (tid, tex.generation));
            }
        }
    }
}
