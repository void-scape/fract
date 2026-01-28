use crate::{byte_slice, config::Config, orbit::Orbit, palette::Palette, ssaa::SsaaPipeline};
use rug::Float;
use std::num::NonZeroU64;

#[repr(C)]
#[derive(Copy, Clone)]
struct MandelbrotUniform {
    iterations: i32,
    zm: f32,
    ze: i32,
    batch_iter: i32,
    palette_len: f32,
    color_scale: f32,
}

/// Perform iterative mandelbrot computation in a compute shader.
///
/// Every frame will increment the orbit of each pixel up to a certain threshold
/// in order to prevent the device from timing out.
pub struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    pixel_state: wgpu::Buffer,
    pixel_state_bytes: u64,
    remaining: wgpu::Buffer,
    remaining_stage: wgpu::Buffer,
}

impl ComputePipeline {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        ssaa: &SsaaPipeline,
        config: &Config,
    ) -> Self {
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<MandelbrotUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let sf = ssaa.ssaa_factor();
        let pixel_state_bytes =
            (std::mem::size_of::<f32>() * 6 * config.width * sf * config.height * sf) as u64;
        let pixel_state = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: pixel_state_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let remaining = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: pixel_state_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let remaining_stage = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: pixel_state_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(ssaa.render_target()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: pixel_state.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: remaining.as_entire_binding(),
                },
            ],
        });

        let module = device.create_shader_module(wgpu::include_wgsl!("shaders/mandelbrot.wgsl"));

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

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants,
                zero_initialize_workgroup_memory: false,
            },
            cache: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform,
            pixel_state,
            pixel_state_bytes,
            remaining,
            remaining_stage,
        }
    }

    pub fn write_buffers(
        &self,
        queue: &wgpu::Queue,
        config: &Config,
        z: &Float,
        palette: &Palette,
    ) {
        let (zm, ze) = z.to_f32_exp();
        queue.write_buffer(
            &self.uniform,
            0,
            byte_slice(&[MandelbrotUniform {
                iterations: config.iterations as i32,
                zm,
                ze,
                batch_iter: config.batch_iter as i32,
                palette_len: palette.len as f32,
                color_scale: config.color_scale,
            }]),
        );
        queue
            .write_buffer_with(
                &self.pixel_state,
                0,
                NonZeroU64::new(self.pixel_state_bytes).unwrap(),
            )
            .unwrap()
            .fill(0);
    }

    pub fn compute_mandelbrot(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        orbit: &Orbit,
        palette: &Palette,
        ssaa: &SsaaPipeline,
        width: usize,
        height: usize,
    ) {
        queue.write_buffer(&self.remaining, 0, &[0; 4]);

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &self.bind_group, &[]);
        cpass.set_bind_group(1, &orbit.bind_group, &[]);
        cpass.set_bind_group(2, &palette.bind_group, &[]);

        let ssaa_factor = ssaa.ssaa_factor();
        let x = (width * ssaa_factor).div_ceil(16) as u32;
        let y = (height * ssaa_factor).div_ceil(16) as u32;
        cpass.dispatch_workgroups(x, y, 1);
    }

    pub fn remaining_pixels(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
    ) -> u32 {
        encoder.copy_buffer_to_buffer(&self.remaining, 0, &self.remaining_stage, 0, 4);
        queue.submit([encoder.finish()]);

        let slice = self.remaining_stage.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let data = slice.get_mapped_range();
        let remaining = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        drop(data);
        self.remaining_stage.unmap();
        remaining
    }
}
