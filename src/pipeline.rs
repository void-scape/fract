use crate::{
    compute::ComputePipeline, orbit::Orbit, palette::Palette, precision, ssaa::SsaaPipeline,
};
use glazer::winit::window::Window;
use rug::Float;
use tint::Sbgr;

pub struct Pipeline {
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    width: usize,
    height: usize,
    //
    output_buffer: wgpu::Buffer,
    bytes_per_row: usize,
    compute: ComputePipeline,
    ssaa: SsaaPipeline,
    orbit: Orbit,
    palette: Palette,
    //
    finished_render: bool,
    updated_position: bool,
    x: Float,
    y: Float,
    z: Float,
}

impl Pipeline {
    pub fn new(
        window: Option<&Window>,
        palette: &[Sbgr],
        width: usize,
        height: usize,
        ssaa: bool,
        x: Float,
        y: Float,
        z: Float,
    ) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = window.map(|window| {
            instance
                .create_surface(window)
                .expect("Failed to create surface")
        });
        // SAFETY: `glazer` must pass the window to the `update_and_render` callback,
        // therefore it will be a valid reference whenever this surface is used.
        let mut surface = unsafe {
            std::mem::transmute::<Option<wgpu::Surface<'_>>, Option<wgpu::Surface<'static>>>(
                surface,
            )
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
            required_features: wgpu::Features::FLOAT32_FILTERABLE,
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

        let ssaa = SsaaPipeline::new(&device, surface_format, width, height, ssaa);
        let compute = ComputePipeline::new(
            &device,
            surface_format,
            ssaa.render_target(),
            width,
            height,
            &ssaa,
        );
        let orbit = Orbit::new(&device);
        let palette = Palette::new(&device, &queue, palette);

        let (bytes_per_row, buffer_size) = output_buffer_bytes_per_row_and_size(width, height);
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
            output_buffer,
            bytes_per_row,
            compute,
            ssaa,
            orbit,
            palette,
            //
            finished_render: false,
            updated_position: true,
            x,
            y,
            z,
        }
    }

    pub fn total_pixels(&self) -> usize {
        let sf = self.ssaa.ssaa_factor();
        self.width * sf * self.height * sf
    }

    pub fn read_position<R>(&mut self, f: impl FnOnce(&Float, &Float, &Float) -> R) -> R {
        f(&self.x, &self.y, &self.z)
    }

    pub fn write_position<R>(
        &mut self,
        f: impl FnOnce(&mut Float, &mut Float, &mut Float) -> R,
    ) -> R {
        self.updated_position = true;
        let result = f(&mut self.x, &mut self.y, &mut self.z);
        let prec = precision(&self.z);
        if self.z.prec() != prec {
            self.z.set_prec(prec);
            self.x.set_prec(prec);
            self.y.set_prec(prec);
        }
        result
    }

    /// Renders pixels with an iteration limit.
    ///
    /// Returns the remaining pixels to render.
    pub fn step_mandelbrot(&mut self, iterations: usize) -> u32 {
        if self.finished() {
            return 0;
        }

        if self.updated_position {
            self.updated_position = false;
            self.orbit
                .compute_reference_orbit(&self.x, &self.y, &self.z, iterations);
            self.orbit.write_buffers(&self.queue, &self.z);
            self.compute.write_buffers(&self.queue, iterations, &self.z);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.compute.compute_mandelbrot(
            &self.queue,
            &mut encoder,
            &self.orbit,
            &self.palette,
            &self.ssaa,
            self.width,
            self.height,
        );
        self.ssaa.render_pass(&mut encoder);
        let remaining = self
            .compute
            .remaining_pixels(&self.device, &self.queue, encoder);
        self.finished_render = remaining == 0;
        remaining
    }

    /// [`Pipeline::step_mandelbrot`] without rendering into the offscreen buffer.
    pub fn step_mandelbrot_headless(&mut self, iterations: usize) -> u32 {
        if self.finished() {
            return 0;
        }

        if self.updated_position {
            self.updated_position = false;
            self.orbit
                .compute_reference_orbit(&self.x, &self.y, &self.z, iterations);
            self.orbit.write_buffers(&self.queue, &self.z);
            self.compute.write_buffers(&self.queue, iterations, &self.z);
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.compute.compute_mandelbrot(
            &self.queue,
            &mut encoder,
            &self.orbit,
            &self.palette,
            &self.ssaa,
            self.width,
            self.height,
        );
        let remaining = self
            .compute
            .remaining_pixels(&self.device, &self.queue, encoder);
        self.finished_render = remaining == 0;
        remaining
    }

    /// Renders the mandelbrot into the offscreen buffer.
    pub fn render_output(&self) {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.ssaa.render_pass(&mut encoder);
        self.queue.submit([encoder.finish()]);
    }

    /// Copy the output buffer into the surface, if one exists.
    pub fn present(&self) {
        let Some(surface) = &self.surface else {
            return;
        };

        let surface_texture = surface.get_current_texture().unwrap();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: self.ssaa.output_texture(),
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
                width: self.width as u32,
                height: self.height as u32,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        surface_texture.present();
    }

    /// [`Pipeline`] has rendered all of the pixels for the current position.
    pub fn finished(&self) -> bool {
        !self.updated_position && self.finished_render
    }

    /// Copy the output buffer into a staging buffer then block while staging
    /// buffer maps to CPU memory.
    ///
    /// Pixel byte format depends on the surface texture.
    pub fn read_output_buffer_bytes(&self) -> Vec<u8> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: self.ssaa.output_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row as u32),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = self.output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .unwrap();

        let padded_data = buffer_slice.get_mapped_range();
        let mut result = Vec::with_capacity(self.width * self.height * 4);
        for chunk in padded_data.chunks(self.bytes_per_row) {
            result.extend_from_slice(&chunk[..self.width * 4]);
        }
        drop(padded_data);
        self.output_buffer.unmap();

        result
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
