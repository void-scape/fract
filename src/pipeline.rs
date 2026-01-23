use crate::{MANDELBROT_XRANGE, MANDELBROT_YRANGE, PRECISION, series_approximation_coefficients};
use glazer::winit::window::Window;
use rug::{Assign, Float, ops::CompleteRound};

pub const ITERATIONS: usize = 10_000;

pub struct Pipeline {
    surface: wgpu::Surface<'static>,
    _config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    //
    pipeline: wgpu::RenderPipeline,
    perturbation_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    orbit_buffer: wgpu::Buffer,
    offscreen_texture: wgpu::Texture,
    offscreen_texture_view: wgpu::TextureView,
    orbit: Vec<(f32, f32)>,
    zoom: Float,
    x: Float,
    y: Float,
}

pub fn create_pipeline(window: &Window) -> Pipeline {
    env_logger::init();

    window.set_resizable(false);
    window.set_title("Mandelbrot Set");

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let surface = instance
        .create_surface(window)
        .expect("Failed to create surface");
    // SAFETY: `glazer` must pass the window to the `update_and_render` callback,
    // therefore it will be a valid reference whenever this surface is used.
    let surface =
        unsafe { std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface) };
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: Some(&surface),
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
        width: crate::WIDTH as u32,
        height: crate::HEIGHT as u32,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<MandelbrotUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let orbit_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<f32>() as u64 * 2 * ITERATIONS as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
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
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&bind_group_layout],
        ..Default::default()
    });

    let perturbation_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            concat!(
                include_str!("color.wgsl"),
                include_str!("perturbation_shader.wgsl")
            )
            .into(),
        ),
    });

    // Metal uses Bgra, so the shader might need to swap the channels.
    let needs_swap = matches!(
        surface_format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    );
    let constants = [("SWAP_CHANNELS", if needs_swap { 1.0 } else { 0.0 })];

    let perturbation_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &perturbation_shader,
            entry_point: Some("vs_main"),
            buffers: &[],
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
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        cache: None,
        multiview_mask: None,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(
            concat!(include_str!("color.wgsl"), include_str!("shader.wgsl")).into(),
        ),
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

    let texture_desc = wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: crate::WIDTH as u32,
            height: crate::HEIGHT as u32,
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

    Pipeline {
        surface,
        _config: config,
        device,
        queue,
        //
        pipeline,
        perturbation_pipeline,
        bind_group,
        uniform_buffer,
        orbit_buffer,
        offscreen_texture,
        offscreen_texture_view,
        orbit: Vec::new(),
        zoom: Float::with_val(PRECISION, 0.0),
        x: Float::with_val(PRECISION, 0.0),
        y: Float::with_val(PRECISION, 0.0),
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
struct MandelbrotUniform {
    width: u32,
    height: u32,
    max_iteration: u32,
    xstep: f32,
    ystep: f32,
    sdx: f32,
    sdy: f32,
    orbit_len: u32,
    zoom: f32,
    cx: f32,
    cy: f32,
    //
    approx_iteration: u32,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    cxx: f32,
    cyy: f32,
}

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

    let cx = x;
    let cy = y;

    let mut encoder = pipeline
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    let surface_texture = pipeline.surface.get_current_texture().unwrap();

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
        pipeline.orbit.push((x.to_f32(), y.to_f32()));
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

    let w = crate::WIDTH as f64;
    let h = crate::HEIGHT as f64;
    let xstep = (Float::with_val(PRECISION, MANDELBROT_XRANGE) * zoom / w).to_f64();
    let ystep = (Float::with_val(PRECISION, MANDELBROT_YRANGE) * zoom / h).to_f64();
    let sdx = (Float::with_val(PRECISION, -2.00) * zoom).to_f64();
    let sdy = (Float::with_val(PRECISION, -1.12) * zoom).to_f64();
    let (a, b, c, approx_iteration) =
        series_approximation_coefficients(&pipeline.orbit, sdx, sdy, xstep, ystep);

    println!("x: {}", cx.to_string_radix(10, Some(50)));
    println!("y: {}", cy.to_string_radix(10, Some(50)));
    println!("z: {}", zoom.to_string_radix(10, Some(50)));
    println!("i: {}", max_iteration);

    println!("a: {}", a);
    println!("b: {}", b);
    println!("c: {}", c);

    println!("approx: {}", approx_iteration);
    println!("orbit len: {}", pipeline.orbit.len());
    println!();

    let args = MandelbrotUniform {
        width: crate::WIDTH as u32,
        height: crate::HEIGHT as u32,
        max_iteration: max_iteration as u32,
        xstep: xstep as f32,
        ystep: ystep as f32,
        sdx: sdx as f32,
        sdy: sdy as f32,
        orbit_len: pipeline.orbit.len() as u32,
        zoom: zoom.to_f32(),
        cx: cx.to_f32(),
        cy: cy.to_f32(),
        approx_iteration: approx_iteration as u32,
        ax: a.re as f32,
        ay: a.im as f32,
        bx: b.re as f32,
        by: b.im as f32,
        cxx: c.re as f32,
        cyy: c.im as f32,
    };

    pipeline
        .queue
        .write_buffer(&pipeline.uniform_buffer, 0, byte_slice(&[args]));
    pipeline
        .queue
        .write_buffer(&pipeline.orbit_buffer, 0, byte_slice(&pipeline.orbit));

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
    // NOTE: When the zoom level is high, the perturbation algorithm I have
    // implemented becomes unstable, so it falls back to the original direct
    // implementation.
    if *zoom < 0.001 {
        rpass.set_pipeline(&pipeline.perturbation_pipeline);
    } else {
        rpass.set_pipeline(&pipeline.pipeline);
    }
    rpass.set_bind_group(0, &pipeline.bind_group, &[]);
    rpass.draw(0..3, 0..1);
    drop(rpass);

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
            height: crate::HEIGHT as u32,
            width: crate::WIDTH as u32,
            depth_or_array_layers: 1,
        },
    );
    pipeline.queue.submit([encoder.finish()]);
    surface_texture.present();
}
