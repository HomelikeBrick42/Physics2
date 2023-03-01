use eframe::wgpu::{self, include_wgsl, util::DeviceExt};
use encase::{ArrayLength, ShaderSize, ShaderType, StorageBuffer, UniformBuffer};

#[derive(Clone, Copy, ShaderType)]
pub struct CameraUniform {
    pub position: cgmath::Vector2<f32>,
    pub rotation: f32,
    pub zoom: f32,
    pub screen_size: cgmath::Vector2<f32>,
}

#[derive(Clone, Copy, ShaderType)]
pub struct StorageBufferQuad {
    pub position: cgmath::Vector2<f32>,
    pub scale: cgmath::Vector2<f32>,
    pub color: cgmath::Vector3<f32>,
    pub rotation: f32,
}

#[derive(Clone, ShaderType)]
pub struct QuadStorageBuffer<'a> {
    pub length: ArrayLength,
    #[size(runtime)]
    pub quads: &'a [StorageBufferQuad],
}

pub(crate) struct Renderer {
    camera_uniform_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    quad_pipeline: wgpu::RenderPipeline,
    quad_storage_buffer: wgpu::Buffer,
    quad_bind_group_layout: wgpu::BindGroupLayout,
    quad_bind_group: wgpu::BindGroup,
    quad_storage_buffer_capacity: usize,
    quad_count: usize,
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) -> Self {
        let camera_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniform Buffer"),
            contents: &[0; CameraUniform::SHADER_SIZE.get() as _],
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(CameraUniform::SHADER_SIZE),
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_uniform_buffer.as_entire_binding(),
            }],
        });

        let quad_storage_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Storage Buffer"),
            contents: &[0; QuadStorageBuffer::METADATA.min_size().get() as _],
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        });

        let quad_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Quad Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: Some(QuadStorageBuffer::METADATA.min_size().0),
                    },
                    count: None,
                }],
            });

        let quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Quad Bind Group"),
            layout: &quad_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: quad_storage_buffer.as_entire_binding(),
            }],
        });

        let quad_shader = device.create_shader_module(include_wgsl!("./quad_shader.wgsl"));

        let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Quad Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &quad_bind_group_layout],
            push_constant_ranges: &[],
        });

        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Quad Pipeline"),
            layout: Some(&quad_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &quad_shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &quad_shader,
                entry_point: "fs_main",
                targets: &[Some(target_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None, // this will be needed if using an index buffer
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None, // culling is not needed
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            camera_uniform_buffer,
            camera_bind_group,
            quad_pipeline,
            quad_storage_buffer,
            quad_bind_group_layout,
            quad_bind_group,
            quad_storage_buffer_capacity: 0,
            quad_count: 0,
        }
    }

    pub fn prepare(
        &mut self,
        camera: CameraUniform,
        quads: &[StorageBufferQuad],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _encoder: &mut wgpu::CommandEncoder,
    ) -> Vec<wgpu::CommandBuffer> {
        {
            let mut buffer = UniformBuffer::new([0; CameraUniform::SHADER_SIZE.get() as _]);
            buffer.write(&camera).unwrap();
            let buffer = buffer.into_inner();
            queue.write_buffer(&self.camera_uniform_buffer, 0, &buffer);
        }

        {
            let quad_storage_buffer_data = QuadStorageBuffer {
                length: ArrayLength,
                quads,
            };

            let mut buffer = StorageBuffer::new(Vec::with_capacity(
                quad_storage_buffer_data.size().get() as _,
            ));
            buffer.write(&quad_storage_buffer_data).unwrap();
            let buffer = buffer.into_inner();
            if buffer.len() > self.quad_storage_buffer_capacity {
                self.quad_storage_buffer =
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Quad Storage Buffer"),
                        contents: &buffer,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                    });

                self.quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Quad Bind Group"),
                    layout: &self.quad_bind_group_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.quad_storage_buffer.as_entire_binding(),
                    }],
                });

                self.quad_storage_buffer_capacity = buffer.len();
            } else {
                queue.write_buffer(&self.quad_storage_buffer, 0, &buffer);
            }
            self.quad_count = quads.len();
        }

        vec![]
    }

    pub fn paint<'rp>(&'rp self, render_pass: &mut wgpu::RenderPass<'rp>) {
        render_pass.set_pipeline(&self.quad_pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, &self.quad_bind_group, &[]);
        render_pass.draw(0..4, 0..self.quad_count as _);
    }
}
