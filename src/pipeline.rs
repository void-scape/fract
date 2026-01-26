use crate::{PRECISION, byte_slice, orbit::Orbit, ssaa::SsaaPipeline};
use glazer::winit::window::Window;
use rug::{Assign, Float};
use tint::Sbgr;
use wgpu::util::DeviceExt;

pub const MAX_ITERATIONS: usize = 100_000;

pub struct Pipeline {
    surface: Option<wgpu::Surface<'static>>,
    pub device: wgpu::Device,
    queue: wgpu::Queue,
    pub width: usize,
    pub height: usize,
    //
    pipeline: wgpu::RenderPipeline,
    ssaa: SsaaPipeline,
    orbit: Orbit,
    vertex_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    pub output_buffers: [wgpu::Buffer; 2],
    pub current_buffer: usize,
    zoom: Float,
    x: Float,
    y: Float,
}

pub fn create_pipeline(
    window: Option<&Window>,
    palette: &[Sbgr],
    width: usize,
    height: usize,
    ssaa: bool,
) -> Pipeline {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let surface = window.map(|window| {
        instance
            .create_surface(window)
            .expect("Failed to create surface")
    });
    // SAFETY: `glazer` must pass the window to the `update_and_render` callback,
    // therefore it will be a valid reference whenever this surface is used.
    let mut surface = unsafe {
        std::mem::transmute::<Option<wgpu::Surface<'_>>, Option<wgpu::Surface<'static>>>(surface)
    };
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: surface.as_ref(),
        force_fallback_adapter: false,
    }))
    .expect("Failed to create adapter");
    println!("Running on Adapter: {:#?}", adapter.get_info());

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: adapter.limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("Failed to create device");

    let surface_format = if let Some(surface) = surface.as_mut() {
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        println!("Surface format: {:?}", surface_format);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: width as u32,
            height: height as u32,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        surface_format
    } else {
        wgpu::TextureFormat::Rgba8UnormSrgb
    };

    let orbit = Orbit::new(&device);

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<MandelbrotUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let palette_texture = palette_texture(&device, &queue, palette);
    let palette_texture_view = palette_texture.create_view(&Default::default());
    let palette_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
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
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
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
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: orbit.uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: orbit.point_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&palette_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Sampler(&palette_sampler),
            },
        ],
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: byte_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let vertex_buffer_layout = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        }],
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
        ..Default::default()
    });

    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/mandelbrot.wgsl"));

    // Metal uses Bgra, so the shader might need to swap the channels.
    let needs_swap = matches!(
        surface_format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    let constants = [("SWAP_CHANNELS", if needs_swap { 1.0 } else { 0.0 })];

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_buffer_layout],
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
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        cache: None,
        multiview_mask: None,
    });

    let (_, buffer_size) = output_buffer_bytes_per_row_and_size(width, height);
    let output_buffer1 = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: buffer_size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let output_buffer2 = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: buffer_size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let ssaa = SsaaPipeline::new(&device, surface_format, width, height, ssaa);

    Pipeline {
        surface,
        device,
        queue,
        width,
        height,
        //
        pipeline,
        ssaa,
        orbit,
        vertex_buffer,
        bind_group,
        uniform_buffer,
        output_buffers: [output_buffer1, output_buffer2],
        current_buffer: 0,
        zoom: Float::with_val(PRECISION, 0.0),
        x: Float::with_val(PRECISION, 0.0),
        y: Float::with_val(PRECISION, 0.0),
    }
}

fn palette_texture(device: &wgpu::Device, queue: &wgpu::Queue, palette: &[Sbgr]) -> wgpu::Texture {
    let palette_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: palette.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &palette_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        byte_slice(palette),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(palette.len() as u32 * 4),
            rows_per_image: None,
        },
        wgpu::Extent3d {
            width: palette.len() as u32,
            height: 1,
            depth_or_array_layers: 1,
        },
    );

    palette_texture
}

#[repr(C)]
#[derive(Copy, Clone)]
struct MandelbrotUniform {
    width: u32,
    height: u32,
    iterations: u32,
    zoom: f32,
    q: i32,
    _pad: [u32; 3],
}

const VERTICES: &[[f32; 2]] = &[[1.0, 1.0], [-1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]];

pub fn compute_mandelbrot(
    pipeline: &mut Pipeline,
    iterations: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    if *zoom == pipeline.zoom && *x == pipeline.x && *y == pipeline.y {
        return;
    }

    pipeline.zoom.assign(zoom);
    pipeline.x.assign(x);
    pipeline.y.assign(y);

    pipeline
        .orbit
        .compute_reference_orbit(x, y, zoom, iterations);

    let (zm, q) = zoom.to_f32_exp();
    let args = MandelbrotUniform {
        width: pipeline.width as u32,
        height: pipeline.height as u32,
        iterations: iterations as u32,
        zoom: zm,
        q,
        _pad: [0; 3],
    };

    pipeline
        .queue
        .write_buffer(&pipeline.uniform_buffer, 0, byte_slice(&[args]));
    pipeline
        .orbit
        .write_buffers(&pipeline.queue, &pipeline.zoom);
    pipeline.queue.submit([]);

    if iterations > 50_000
        || (pipeline.width * pipeline.height >= 1000 * 1000 && pipeline.ssaa.enabled())
        || (pipeline.width * pipeline.height >= 2560 * 1440)
    {
        conservative_render_pass(pipeline);
    } else {
        let mut encoder = pipeline
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        // NOTE: Being very cautious here about the watch dog timer killing this render,
        // so this gets its own queue.
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: pipeline.ssaa.render_target(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });
        rpass.set_pipeline(&pipeline.pipeline);
        rpass.set_bind_group(0, &pipeline.bind_group, &[]);
        rpass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
        rpass.draw(0..4, 0..1);
        drop(rpass);
        pipeline.queue.submit([encoder.finish()]);
    }

    let mut encoder = pipeline
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    pipeline.ssaa.render_pass(&mut encoder);

    if let Some(surface) = &pipeline.surface {
        let surface_texture = surface.get_current_texture().unwrap();
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: pipeline.ssaa.output_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &surface_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: pipeline.width as u32,
                height: pipeline.height as u32,
                depth_or_array_layers: 1,
            },
        );
        pipeline.queue.submit([encoder.finish()]);
        surface_texture.present();
        return;
    }
    pipeline.queue.submit([encoder.finish()]);
}

// NOTE: Being VERY cautious here about the watch dog timer killing this render,
// so this gets split into little tiny blocks.
//
// Might just use this by default...
fn conservative_render_pass(pipeline: &mut Pipeline) {
    let width = pipeline.ssaa.ssaa_dimension(pipeline.width);
    let height = pipeline.ssaa.ssaa_dimension(pipeline.height);

    let tile_size = 64;
    let xtiles = width.div_ceil(tile_size);
    let ytiles = height.div_ceil(tile_size);

    for ty in 0..ytiles {
        for tx in 0..xtiles {
            let x = (tx * tile_size) as u32;
            let y = (ty * tile_size) as u32;

            let w = (tile_size as u32).min(width as u32 - x);
            let h = (tile_size as u32).min(height as u32 - y);

            let mut encoder = pipeline
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: pipeline.ssaa.render_target(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });

            rpass.set_pipeline(&pipeline.pipeline);
            rpass.set_bind_group(0, &pipeline.bind_group, &[]);
            rpass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
            rpass.set_scissor_rect(x, y, w, h);
            rpass.draw(0..4, 0..1);
            drop(rpass);

            pipeline.queue.submit(Some(encoder.finish()));
        }
    }
}

fn output_buffer_bytes_per_row_and_size(width: usize, height: usize) -> (usize, usize) {
    let bytes_per_pixel = 4;
    let align = 256;
    let bpr = width * bytes_per_pixel;
    let padding = (align - bpr % align) % align;
    let bpr = bpr + padding;
    let buffer_size = bpr * height;
    (bpr, buffer_size)
}

pub fn stage_frame_pixel_bytes(pipeline: &Pipeline) {
    let mut encoder = pipeline
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let (bytes_per_row, _) = output_buffer_bytes_per_row_and_size(pipeline.width, pipeline.height);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: pipeline.ssaa.output_texture(),
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &pipeline.output_buffers[pipeline.current_buffer],
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row as u32),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: pipeline.width as u32,
            height: pipeline.height as u32,
            depth_or_array_layers: 1,
        },
    );
    pipeline.queue.submit(Some(encoder.finish()));
}

pub fn frame_pixel_bytes(
    device: &wgpu::Device,
    output_buffer: &wgpu::Buffer,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let buffer_slice = output_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();

    let (bytes_per_row, _) = output_buffer_bytes_per_row_and_size(width, height);
    let padded_data = buffer_slice.get_mapped_range();
    let mut result = Vec::with_capacity(width * height * 4);
    for chunk in padded_data.chunks(bytes_per_row) {
        result.extend_from_slice(&chunk[..width * 4]);
    }
    drop(padded_data);
    output_buffer.unmap();
    result
}

pub fn pipeline_frame_pixel_bytes(pipeline: &Pipeline) -> Vec<u8> {
    stage_frame_pixel_bytes(pipeline);
    frame_pixel_bytes(
        &pipeline.device,
        &pipeline.output_buffers[pipeline.current_buffer],
        pipeline.width,
        pipeline.height,
    )
}
