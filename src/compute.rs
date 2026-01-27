use crate::{byte_slice, orbit::Orbit, palette::Palette, ssaa::SsaaPipeline};
use rug::Float;

#[repr(C)]
#[derive(Copy, Clone)]
struct MandelbrotUniform {
    iterations: u32,
    zm: f32,
    ze: i32,
    width: f32,
    height: f32,
}

/// Renders the mandelbrot into a texture.
pub struct ComputePipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}

impl ComputePipeline {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<MandelbrotUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/mandelbrot.wgsl"));

        // Metal uses Bgra, so the shader might need to swap the channels.
        let needs_swap = matches!(
            surface_format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let constants = [("SWAP_CHANNELS", if needs_swap { 1.0 } else { 0.0 })];

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &bind_group_layout,
                &Orbit::bind_group_layout(device),
                &Palette::bind_group_layout(device),
            ],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(surface_format.into())],
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform,
        }
    }

    pub fn write_buffers(
        &self,
        queue: &wgpu::Queue,
        iterations: usize,
        z: &Float,
        width: usize,
        height: usize,
    ) {
        let (zm, ze) = z.to_f32_exp();
        queue.write_buffer(
            &self.uniform,
            0,
            byte_slice(&[MandelbrotUniform {
                iterations: iterations as u32,
                zm,
                ze,
                width: width as f32,
                height: height as f32,
            }]),
        );
    }

    pub fn compute_mandelbrot(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        orbit: &Orbit,
        palette: &Palette,
        ssaa: &SsaaPipeline,
    ) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ssaa.render_target(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.set_bind_group(1, &orbit.bind_group, &[]);
        rpass.set_bind_group(2, &palette.bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
}
