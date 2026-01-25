use crate::PRECISION;
use glazer::winit::window::Window;
use rug::{
    Assign, Float,
    ops::{CompleteRound, Pow, PowAssign},
};
use tint::Sbgr;
use wgpu::util::DeviceExt;

pub const MAX_ITERATIONS: usize = 100_000;

pub struct Pipeline {
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    width: usize,
    height: usize,
    //
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    orbit_buffer: wgpu::Buffer,
    offscreen_texture: wgpu::Texture,
    offscreen_texture_view: wgpu::TextureView,
    output_buffer: wgpu::Buffer,
    orbit: Vec<OrbitDelta>,
    zoom: Float,
    x: Float,
    y: Float,
}

#[repr(C)]
struct OrbitDelta {
    dx: f32,
    dy: f32,
    exponent: i32,
    _padding: u32,
}

pub fn create_pipeline(
    window: Option<&Window>,
    palette: &[Sbgr],
    width: usize,
    height: usize,
) -> Pipeline {
    env_logger::init();

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
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: surface.as_ref(),
        force_fallback_adapter: false,
    }))
    .expect("Failed to create adapter");
    println!("Running on Adapter: {:#?}", adapter.get_info());

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
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

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<MandelbrotUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let orbit_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<OrbitDelta>() as u64 * MAX_ITERATIONS as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
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
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
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
                resource: orbit_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&palette_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
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

    let perturbation_shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

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
            module: &perturbation_shader,
            entry_point: Some("vs_main"),
            buffers: &[vertex_buffer_layout],
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants,
                ..Default::default()
            },
        },
        fragment: Some(wgpu::FragmentState {
            module: &perturbation_shader,
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

    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: width as u32,
            height: height as u32,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: surface_format,
        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
        label: None,
        view_formats: &[],
    };
    let offscreen_texture = device.create_texture(&texture_desc);
    let offscreen_texture_view = offscreen_texture.create_view(&Default::default());

    let (_, buffer_size) = output_buffer_bytes_per_row_and_size(width, height);
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: buffer_size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    Pipeline {
        surface,
        device,
        queue,
        width,
        height,
        //
        pipeline,
        vertex_buffer,
        bind_group,
        uniform_buffer,
        orbit_buffer,
        offscreen_texture,
        offscreen_texture_view,
        output_buffer,
        orbit: Vec::new(),
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
    max_iteration: u32,
    orbit_len: u32,
    zoom: f32,
    q: i32,
    _pad: [u32; 2], //
                    // approx_iteration: u32,
                    // ax: f32,
                    // ay: f32,
                    // bx: f32,
                    // by: f32,
                    // cxx: f32,
                    // cyy: f32,
}

const VERTICES: &[[f32; 2]] = &[[1.0, 1.0], [-1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]];

fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

pub fn compute_mandelbrot(
    pipeline: &mut Pipeline,
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    if *zoom == pipeline.zoom && *x == pipeline.x && *y == pipeline.y {
        return;
    }

    let mut encoder = pipeline
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    pipeline.zoom.assign(zoom);
    pipeline.x.assign(x);
    pipeline.y.assign(y);

    pipeline.orbit.clear();
    let x0 = x;
    let y0 = y;
    let mut x = Float::with_val(PRECISION, 0.0);
    let mut y = Float::with_val(PRECISION, 0.0);
    let mut x2 = Float::with_val(PRECISION, 0.0);
    let mut y2 = Float::with_val(PRECISION, 0.0);
    let mut xy = Float::with_val(PRECISION, 0.0);

    for _ in 0..max_iteration {
        let x_exp = x.get_exp().unwrap_or(0);
        let y_exp = y.get_exp().unwrap_or(0);

        let scale_exp = x_exp.max(y_exp);
        let scale_exp = if scale_exp < -10000 { 0 } else { scale_exp };
        let x_shifted = &x / Float::with_val(PRECISION, 2.0).pow(scale_exp);
        let x_mantissa = x_shifted.to_f32();

        let y_shifted = &y / Float::with_val(PRECISION, 2.0).pow(scale_exp);
        let y_mantissa = y_shifted.to_f32();

        pipeline.orbit.push(OrbitDelta {
            dx: x_mantissa,
            dy: y_mantissa,
            exponent: scale_exp,
            _padding: 0,
        });

        x2.assign(&x * &x);
        y2.assign(&y * &y);
        if (&x2 + &y2).complete(PRECISION) > 4.0 {
            break;
        }
        xy.assign(&x * &y);
        y.assign(&xy * 2.0);
        y += y0;
        x.assign(&x2 - &y2);
        x += x0;
    }

    let q = zoom.get_exp().unwrap_or(0);
    let mut denom = Float::with_val(PRECISION, 2.0);
    denom.pow_assign(q);
    let zoom = Float::with_val(PRECISION, zoom / denom);

    let args = MandelbrotUniform {
        width: pipeline.width as u32,
        height: pipeline.height as u32,
        max_iteration: max_iteration as u32,
        orbit_len: pipeline.orbit.len() as u32,
        zoom: zoom.to_f32(),
        q,
        _pad: [0, 0],
        //
        // approx_iteration: approx_iteration as u32,
        // ax: a.re as f32,
        // ay: a.im as f32,
        // bx: b.re as f32,
        // by: b.im as f32,
        // cxx: c.re as f32,
        // cyy: c.im as f32,
    };

    let mut staging_belt = wgpu::util::StagingBelt::new(pipeline.device.clone(), 1024);
    staging_belt
        .write_buffer(
            &mut encoder,
            &pipeline.uniform_buffer,
            0,
            wgpu::BufferSize::new(std::mem::size_of::<MandelbrotUniform>() as u64).unwrap(),
        )
        .copy_from_slice(byte_slice(&[args]));
    staging_belt
        .write_buffer(
            &mut encoder,
            &pipeline.orbit_buffer,
            0,
            wgpu::BufferSize::new(
                (pipeline.orbit.len() * std::mem::size_of::<OrbitDelta>()) as u64,
            )
            .unwrap(),
        )
        .copy_from_slice(byte_slice(&pipeline.orbit));

    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &pipeline.offscreen_texture_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    rpass.set_pipeline(&pipeline.pipeline);
    rpass.set_bind_group(0, &pipeline.bind_group, &[]);
    rpass.set_vertex_buffer(0, pipeline.vertex_buffer.slice(..));
    rpass.draw(0..4, 0..1);
    drop(rpass);

    if let Some(surface) = &pipeline.surface {
        let surface_texture = surface.get_current_texture().unwrap();
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &pipeline.offscreen_texture,
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
        staging_belt.finish();
        pipeline.queue.submit([encoder.finish()]);
        staging_belt.recall();
        surface_texture.present();
        return;
    }

    staging_belt.finish();
    pipeline.queue.submit([encoder.finish()]);
    staging_belt.recall();
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

pub fn frame_pixel_bytes(pipeline: &mut Pipeline) -> Vec<u8> {
    let mut encoder = pipeline
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let (bytes_per_row, _) = output_buffer_bytes_per_row_and_size(pipeline.width, pipeline.height);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &pipeline.offscreen_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &pipeline.output_buffer,
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

    let buffer_slice = pipeline.output_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    pipeline
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .unwrap();

    let padded_data = buffer_slice.get_mapped_range();
    let mut result = Vec::with_capacity(pipeline.width * pipeline.height * 4);
    for chunk in padded_data.chunks(bytes_per_row) {
        result.extend_from_slice(&chunk[..pipeline.width * 4]);
    }
    drop(padded_data);
    pipeline.output_buffer.unmap();
    result
}
