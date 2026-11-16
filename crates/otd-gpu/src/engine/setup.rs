use std::collections::{HashMap, HashSet};

use slotmap::SecondaryMap;
use wgpu::util::DeviceExt;

use crate::context::GpuContext;
use crate::texture::{TOP_FORMAT, TexturePool};

use super::TopEngine;

impl TopEngine {
    pub fn new(ctx: GpuContext) -> Self {
        let device = &ctx.device;

        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("otd top bind layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("otd top pipeline layout"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });

        let make_sampler = |filter, label| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::MipmapFilterMode::Nearest,
                ..Default::default()
            })
        };
        let sampler_linear = make_sampler(wgpu::FilterMode::Linear, "otd sampler linear");
        let sampler_nearest = make_sampler(wgpu::FilterMode::Nearest, "otd sampler nearest");

        let dummy_tex = device.create_texture_with_data(
            &ctx.queue,
            &wgpu::TextureDescriptor {
                label: Some("otd dummy input"),
                size: wgpu::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: TOP_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            wgpu::util::TextureDataOrder::LayerMajor,
            &[0u8; 8],
        );
        let dummy = dummy_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_table_tex = device.create_texture_with_data(
            &ctx.queue,
            &wgpu::TextureDescriptor {
                label: Some("otd const table dummy"),
                size: wgpu::Extent3d {
                    width: 1,
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
            bytemuck::bytes_of(&0f32),
        );
        let dummy_table = dummy_table_tex.create_view(&wgpu::TextureViewDescriptor::default());

        TopEngine {
            ctx,
            bind_layout,
            pipeline_layout,
            sampler_linear,
            sampler_nearest,
            dummy,
            dummy_table,
            const_tables: HashMap::new(),
            table_override: None,
            pipelines: HashMap::new(),
            failed: HashMap::new(),
            glsl_vertex: None,
            nodes: SecondaryMap::new(),
            videos: SecondaryMap::new(),
            soundtracks: SecondaryMap::new(),
            pool: TexturePool::new(),
            encoder: None,
            scratch: Vec::new(),
            renderer: None,
            recorders: SecondaryMap::new(),
            pending_records: Vec::new(),
            pending_feedback: Vec::new(),
            feedback_snapshot: SecondaryMap::new(),
            chop_pixel_cache: HashMap::new(),
            chop_pixel_wanted: HashSet::new(),
            chop_pixel_pending: HashMap::new(),
            fonts: crate::text::FontCache::default(),
            text_keys: SecondaryMap::new(),
            movie_ui: SecondaryMap::new(),
            movie_out_edge: SecondaryMap::new(),
            movie_out_seq: SecondaryMap::new(),
            movie_out_stills: SecondaryMap::new(),
            passes_this_frame: 0,
            sop_previews: SecondaryMap::new(),
            defer_submit: false,
            deferred_gpu: Vec::new(),
        }
    }
}