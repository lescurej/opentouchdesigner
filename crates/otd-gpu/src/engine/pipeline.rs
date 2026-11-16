use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use wgpu::util::DeviceExt;

use otd_core::{CookContext, CookError, Graph};

use crate::ops;
use crate::shader;
use crate::texture::{TOP_FORMAT, TopTexture};

use super::TopEngine;

impl TopEngine {
    fn build_pipeline_format(
        &self,
        label: &str,
        vertex: &wgpu::ShaderModule,
        fragment: &wgpu::ShaderModule,
        entry: &str,
        color_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let (vertex_entry, fragment_entry) = if entry == "main" {
            ("main", "main")
        } else {
            ("vs_main", entry)
        };
        self.ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vertex,
                    entry_point: Some(vertex_entry),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: fragment,
                    entry_point: Some(fragment_entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
    }

    pub(super) fn build_pipeline(
        &self,
        label: &str,
        vertex: &wgpu::ShaderModule,
        fragment: &wgpu::ShaderModule,
        entry: &str,
    ) -> wgpu::RenderPipeline {
        self.build_pipeline_format(label, vertex, fragment, entry, TOP_FORMAT)
    }

    const DISPLAY_BLIT_KEY: &'static str = "_egui_rgba8_blit";

    fn ensure_display_blit_pipeline(&mut self) -> Result<&wgpu::RenderPipeline, CookError> {
        if !self.pipelines.contains_key(Self::DISPLAY_BLIT_KEY) {
            let spec = ops::spec(ops::NULL)
                .ok_or_else(|| CookError::op("display blit", "no null shader"))?;
            let source = format!("{}\n{}", ops::COMMON_WGSL, spec.shader);
            let module = self
                .ctx
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("egui display blit"),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            let pipe = self.build_pipeline_format(
                "egui display blit",
                &module,
                &module,
                "fs_main",
                wgpu::TextureFormat::Rgba8Unorm,
            );
            self.pipelines.insert(Self::DISPLAY_BLIT_KEY.into(), pipe);
        }
        Ok(self.pipelines.get(Self::DISPLAY_BLIT_KEY).unwrap())
    }

    pub fn blit_to_rgba8_unorm(
        &mut self,
        graph: &Graph,
        src: &TopTexture,
        dst: &wgpu::TextureView,
        ctx: &CookContext,
    ) -> Result<(), CookError> {
        let id = graph.root();
        let uniforms = self.uniforms(src.key.width, src.key.height, ctx, ops::NO_PARAMS);
        let buf = self.write_uniform(id, 0, uniforms);
        let dummy = self.dummy.clone();
        let owned_frame = self.encoder.is_none();
        if owned_frame {
            self.begin_frame();
        }
        let pipeline = self.ensure_display_blit_pipeline()?.clone();
        let sampler = self.sampler_linear.clone();
        let bind = self
            .ctx
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("egui display blit"),
                layout: &self.bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&src.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&dummy),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&self.dummy_table),
                    },
                ],
            });
        let encoder = self
            .encoder
            .as_mut()
            .ok_or_else(|| CookError::op("display blit", "no encoder"))?;
        {
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui display blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst,
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
        if owned_frame {
            self.end_frame_immediate();
        }
        Ok(())
    }

    pub(super) fn builtin_pipeline(
        &mut self,
        type_name: &str,
    ) -> Result<&wgpu::RenderPipeline, CookError> {
        if !self.pipelines.contains_key(type_name) {
            let spec = ops::spec(type_name)
                .ok_or_else(|| CookError::op(type_name, "no shader for this operator"))?;
            let source = format!("{}\n{}", ops::COMMON_WGSL, spec.shader);
            let module = self
                .ctx
                .device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(type_name),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
            let pipeline = self.build_pipeline(type_name, &module, &module, "fs_main");
            self.pipelines.insert(type_name.to_string(), pipeline);
        }
        Ok(&self.pipelines[type_name])
    }

    fn glsl_vertex_module(&mut self) -> wgpu::ShaderModule {
        if self.glsl_vertex.is_none() {
            self.glsl_vertex = Some(self.ctx.device.create_shader_module(
                wgpu::ShaderModuleDescriptor {
                    label: Some("otd glsl vertex"),
                    source: wgpu::ShaderSource::Glsl {
                        shader: shader::VERTEX_GLSL.into(),
                        stage: wgpu::naga::ShaderStage::Vertex,
                        defines: &[],
                    },
                },
            ));
        }
        self.glsl_vertex.clone().unwrap()
    }

    fn store_const_table(&mut self, key: &str, values: &[f32]) {
        let width = values.len().max(1) as u32;
        let tex = self.ctx.device.create_texture_with_data(
            &self.ctx.queue,
            &wgpu::TextureDescriptor {
                label: Some("otd const table"),
                size: wgpu::Extent3d {
                    width,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R32Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(values),
        );
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.const_tables.insert(key.to_string(), view);
    }

    pub(crate) fn user_pipeline(&mut self, source: &str, is_glsl: bool) -> Result<String, String> {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        is_glsl.hash(&mut hasher);
        let key = format!("user:{:016x}", hasher.finish());

        if self.pipelines.contains_key(&key) {
            return Ok(key);
        }
        if let Some(err) = self.failed.get(&key) {
            return Err(err.clone());
        }

        let result = if is_glsl {
            let (full, table) = shader::wrap_glsl_full(source);
            let vertex = self.glsl_vertex_module();
            shader::validate_glsl(&full).map(|()| {
                if !table.is_empty() {
                    self.store_const_table(&key, &table);
                }
                let module = self
                    .ctx
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(&key),
                        source: wgpu::ShaderSource::Glsl {
                            shader: full.into(),
                            stage: wgpu::naga::ShaderStage::Fragment,
                            defines: &[],
                        },
                    });
                self.build_pipeline(&key, &vertex, &module, "main")
            })
        } else {
            let full = shader::wrap_wgsl(source);
            shader::validate_wgsl(&full).map(|()| {
                let module = self
                    .ctx
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(&key),
                        source: wgpu::ShaderSource::Wgsl(full.into()),
                    });
                self.build_pipeline(&key, &module, &module, "fs_main")
            })
        };

        match result {
            Ok(pipeline) => {
                self.pipelines.insert(key.clone(), pipeline);
                Ok(key)
            }
            Err(e) => {
                self.failed.insert(key, e.clone());
                Err(e)
            }
        }
    }
}