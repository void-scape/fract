use rast::tint::Srgb;
use rug::Float;

pub struct Pipeline {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    output_texture: wgpu::Texture,
    download_buffer: wgpu::Buffer,
}

pub fn create_pipeline() -> Pipeline {
    env_logger::init();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("Failed to create adapter");
    println!("Running on Adapter: {:#?}", adapter.get_info());

    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    if !downlevel_capabilities
        .flags
        .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    {
        panic!("Adapter does not support compute shaders");
    }

    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: None,
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .expect("Failed to create device");

    let module = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: std::mem::size_of::<MandelbrotUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let texture_extent = wgpu::Extent3d {
        width: crate::WIDTH as u32,
        height: crate::HEIGHT as u32,
        depth_or_array_layers: 1,
    };
    let output_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: texture_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (crate::WIDTH * crate::HEIGHT * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    view_dimension: wgpu::TextureViewDimension::D2,
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
                resource: wgpu::BindingResource::TextureView(
                    &output_texture.create_view(&wgpu::TextureViewDescriptor::default()),
                ),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        immediate_size: 0,
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some("mandelbrot"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    Pipeline {
        device,
        queue,
        pipeline,
        uniform_buffer,
        output_texture,
        bind_group,
        download_buffer,
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MandelbrotUniform {
    pub width: u32,
    pub height: u32,
    pub max_iteration: u32,
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
}

fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

pub fn compute_mandelbrot(
    pipeline: &mut Pipeline,
    frame_buffer: &mut [Srgb],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    let args = MandelbrotUniform {
        width: crate::WIDTH as u32,
        height: crate::HEIGHT as u32,
        max_iteration: max_iteration as u32,
        zoom: zoom.to_f32(),
        x: x.to_f32(),
        y: y.to_f32(),
    };

    pipeline
        .queue
        .write_buffer(&pipeline.uniform_buffer, 0, byte_slice(&[args]));

    let mut encoder = pipeline
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
        label: None,
        timestamp_writes: None,
    });
    compute_pass.set_pipeline(&pipeline.pipeline);
    compute_pass.set_bind_group(0, &pipeline.bind_group, &[]);

    compute_pass.dispatch_workgroups(
        crate::WIDTH.div_ceil(8) as u32,
        crate::HEIGHT.div_ceil(8) as u32,
        1,
    );
    drop(compute_pass);

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &pipeline.output_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &pipeline.download_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(crate::WIDTH as u32 * 4),
                rows_per_image: Some(crate::HEIGHT as u32),
            },
        },
        wgpu::Extent3d {
            width: crate::WIDTH as u32,
            height: crate::HEIGHT as u32,
            depth_or_array_layers: 1,
        },
    );

    pipeline.queue.submit([encoder.finish()]);

    // We now map the download buffer so we can read it. Mapping tells wgpu that we want to read/write
    // to the buffer directly by the CPU and it should not permit any more GPU operations on the buffer.
    //
    // Mapping requires that the GPU be finished using the buffer before it resolves, so mapping has a callback
    // to tell you when the mapping is complete.
    let buffer_slice = pipeline.download_buffer.slice(..);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {
        // In this case we know exactly when the mapping will be finished,
        // so we don't need to do anything in the callback.
    });

    // Wait for the GPU to finish working on the submitted work. This doesn't work on WebGPU, so we would need
    // to rely on the callback to know when the buffer is mapped.
    pipeline
        .device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let data = buffer_slice.get_mapped_range();
    let result: &[u32] =
        unsafe { std::slice::from_raw_parts(data.as_ptr().cast(), data.len() / 4) };
    assert_eq!(result.len(), frame_buffer.len());

    let frame_buffer = unsafe { std::mem::transmute::<&mut [Srgb], &mut [u32]>(frame_buffer) };
    frame_buffer.copy_from_slice(result);

    drop(data);
    pipeline.download_buffer.unmap();
}
