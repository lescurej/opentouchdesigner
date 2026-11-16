use super::scene::triangle_edges;
use super::types::{DEPTH_FORMAT, SceneDescription, SceneUniforms};
use crate::scene;
use crate::texture::TOP_FORMAT;
use otd_core::NodeId;
use wgpu::util::DeviceExt;

// ------------------------------------------------------------- the pass

pub struct Renderer {
    pipelines: std::collections::HashMap<(bool, u8), wgpu::RenderPipeline>,
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    module: wgpu::ShaderModule,
    sampler: wgpu::Sampler,
    pub draws_this_frame: u32,
    pub instances_this_frame: u32,
}

const VERTEX_STRIDE: u64 = 12 * 4;
const INSTANCE_STRIDE: u64 = 13 * 4;

impl Renderer {
    pub fn new(device: &wgpu::Device) -> Renderer {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("otd render3d bind layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("otd render3d pipeline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("otd render3d"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/render3d.wgsl").into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("otd render3d sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Renderer {
            pipelines: std::collections::HashMap::new(),
            layout,
            pipeline_layout,
            module,
            sampler,
            draws_this_frame: 0,
            instances_this_frame: 0,
        }
    }

    fn pipeline(
        &mut self,
        device: &wgpu::Device,
        lines: bool,
        cull: Option<wgpu::Face>,
    ) -> &wgpu::RenderPipeline {
        let key = (
            lines,
            match cull {
                Some(wgpu::Face::Back) => 0u8,
                Some(wgpu::Face::Front) => 1,
                None => 2,
            },
        );
        self.pipelines.entry(key).or_insert_with(|| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("otd render3d"),
                layout: Some(&self.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &self.module,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: VERTEX_STRIDE,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![
                                0 => Float32x3, 1 => Float32x3, 2 => Float32x2, 3 => Float32x4
                            ],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: INSTANCE_STRIDE,
                            step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![
                                4 => Float32x3, 5 => Float32x3, 6 => Float32x3, 7 => Float32x4
                            ],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    topology: if lines {
                        wgpu::PrimitiveTopology::LineList
                    } else {
                        wgpu::PrimitiveTopology::TriangleList
                    },
                    cull_mode: if lines { None } else { cull },
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &self.module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: TOP_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        })
    }

    /// Record the scene into an encoder.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        description: &SceneDescription,
        dummy_texture: &wgpu::TextureView,
        map_of: &dyn Fn(NodeId) -> Option<wgpu::TextureView>,
    ) {
        // Buffers and bind groups are built before the pass, because a render
        // pass borrows the encoder for its whole life.
        struct Prepared {
            vertices: wgpu::Buffer,
            indices: Option<(wgpu::Buffer, u32)>,
            instances: wgpu::Buffer,
            vertex_count: u32,
            instance_count: u32,
            bind: wgpu::BindGroup,
            lines: bool,
            cull: Option<wgpu::Face>,
        }

        let mut prepared = Vec::with_capacity(description.items.len());
        for item in &description.items {
            if item.instances.is_empty() {
                continue;
            }
            let vertex_data = item.geometry.vertex_bytes();
            if vertex_data.is_empty() {
                continue;
            }
            let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("otd geometry"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });
            // Drawing a triangle index list as a line list would pair the
            // indices up as they come — a-b, c-a, c-d — which silently loses
            // one edge of every triangle. Expanding to real edge pairs is the
            // difference between a wireframe and a mesh with holes in it.
            // Per item as well as per render: a Wireframe MAT on one object
            // in a lit scene is the useful case, and the Render TOP's own
            // flag is the "show me everything as edges" override.
            // A sprite is a picture; drawing its quad's edges is never what
            // was meant, and a billboard has no back to cull.
            let sprite = item.shading == scene::Shading::PointSprite;
            let lines = !sprite
                && (description.wireframe
                    || item.wireframe
                    || item.geometry.topology != otd_sop::Topology::Triangles);
            let cull = if sprite { None } else { description.cull };
            let edges = (lines && item.geometry.topology == otd_sop::Topology::Triangles)
                .then(|| triangle_edges(&item.geometry));

            let index_data: Option<&[u32]> = match (&edges, item.geometry.indices.is_empty()) {
                (Some(e), _) => Some(e),
                (None, false) => Some(&item.geometry.indices),
                (None, true) => None,
            };
            let indices = index_data.map(|data| {
                (
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("otd geometry indices"),
                        contents: bytemuck::cast_slice(data),
                        usage: wgpu::BufferUsages::INDEX,
                    }),
                    data.len() as u32,
                )
            });
            let instances = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("otd instances"),
                contents: bytemuck::cast_slice(&item.instances),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let uniforms = SceneUniforms {
                view_proj: description.view_proj,
                model: item.model,
                camera: [
                    description.camera_pos[0],
                    description.camera_pos[1],
                    description.camera_pos[2],
                    0.0,
                ],
                camera_right: [
                    description.camera_right[0],
                    description.camera_right[1],
                    description.camera_right[2],
                    0.0,
                ],
                camera_up: [
                    description.camera_up[0],
                    description.camera_up[1],
                    description.camera_up[2],
                    0.0,
                ],
                light_dir: description.light_dir,
                light_color: description.light_color,
                base_color: item.base_color,
                material: item.material,
                render: [
                    description.ambient,
                    1.0,
                    item.shading as u8 as f32,
                    if description.depth { 1.0 } else { 0.0 },
                ],
                depth_range: [description.depth_near, description.depth_far, 0.0, 0.0],
            };
            let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("otd render3d uniforms"),
                contents: bytemuck::bytes_of(&uniforms),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let map = item
                .color_map
                .and_then(map_of)
                .unwrap_or_else(|| dummy_texture.clone());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("otd render3d"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&map),
                    },
                ],
            });

            prepared.push(Prepared {
                vertices,
                indices,
                instances,
                vertex_count: item.geometry.num_vertices() as u32,
                instance_count: item.instances.len() as u32,
                bind,
                lines,
                cull,
            });
        }

        // Pipelines are created up front for the same borrow reason.
        let pipelines: Vec<wgpu::RenderPipeline> = prepared
            .iter()
            .map(|p| self.pipeline(device, p.lines, p.cull).clone())
            .collect();

        let bg = description.background;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("otd render3d"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg[0] as f64,
                        g: bg[1] as f64,
                        b: bg[2] as f64,
                        a: bg[3] as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        for (item, pipeline) in prepared.iter().zip(&pipelines) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &item.bind, &[]);
            pass.set_vertex_buffer(0, item.vertices.slice(..));
            pass.set_vertex_buffer(1, item.instances.slice(..));
            match &item.indices {
                Some((buffer, count)) => {
                    pass.set_index_buffer(buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..*count, 0, 0..item.instance_count);
                }
                None => pass.draw(0..item.vertex_count, 0..item.instance_count),
            }
            self.draws_this_frame += 1;
            self.instances_this_frame += item.instance_count;
        }
    }

    pub fn begin_frame(&mut self) {
        self.draws_this_frame = 0;
        self.instances_this_frame = 0;
    }
}

/// A depth texture matched to a colour target.
pub fn depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("otd depth"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}
