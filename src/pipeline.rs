use crate::{
    compute::ComputePipeline,
    config::Config,
    float_from_str,
    orbit::Orbit,
    palette::{Palette, parse_palette},
    ssaa::SsaaPipeline,
};
use glazer::winit::window::Window;
use malachite_float::Float;
#[cfg(target_arch = "wasm32")]
use std::{cell::RefCell, rc::Rc};

#[cfg(target_arch = "wasm32")]
pub struct PipelineBuilder {
    pipeline: Rc<RefCell<Option<Pipeline>>>,
}

#[cfg(target_arch = "wasm32")]
impl PipelineBuilder {
    pub fn new(window: &Window, config: Config) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window)
            .expect("Failed to create surface");
        // SAFETY: `glazer` must pass the window to the `update_and_render` callback,
        // therefore it will be a valid reference whenever this surface is used.
        let surface =
            unsafe { std::mem::transmute::<wgpu::Surface<'_>, wgpu::Surface<'static>>(surface) };

        let pipeline = Rc::new(RefCell::new(None));
        wasm_bindgen_futures::spawn_local({
            let pipeline = pipeline.clone();
            async move {
                let adapter = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    })
                    .await
                    .unwrap();

                let (device, queue) = adapter
                    .request_device(&wgpu::DeviceDescriptor {
                        label: None,
                        required_features: wgpu::Features::FLOAT32_FILTERABLE,
                        required_limits: adapter.limits(),
                        experimental_features: wgpu::ExperimentalFeatures::disabled(),
                        memory_hints: wgpu::MemoryHints::MemoryUsage,
                        trace: wgpu::Trace::Off,
                    })
                    .await
                    .expect("Failed to create device");

                _ = pipeline.borrow_mut().insert(Pipeline::from_components(
                    adapter,
                    device,
                    queue,
                    Some(surface),
                    config,
                    None,
                ));
            }
        });

        Self { pipeline }
    }

    pub fn poll(&self) -> bool {
        self.pipeline.borrow().is_some()
    }

    pub fn build(self) -> Pipeline {
        self.pipeline.borrow_mut().take().unwrap()
    }
}

pub struct Pipeline {
    surface: Option<wgpu::Surface<'static>>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: Config,
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
    //
    log: Option<Box<dyn std::io::Write>>,
}

impl Pipeline {
    pub fn new(
        window: Option<&Window>,
        config: Config,
        log: Option<Box<dyn std::io::Write>>,
    ) -> Self {
        env_logger::init();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = window.map(|window| {
            instance
                .create_surface(window)
                .expect("Failed to create surface")
        });
        // SAFETY: `glazer` must pass the window to the `update_and_render` callback,
        // therefore it will be a valid reference whenever this surface is used.
        let surface = unsafe {
            std::mem::transmute::<Option<wgpu::Surface<'_>>, Option<wgpu::Surface<'static>>>(
                surface,
            )
        };
        let mut adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: false,
            }));

        if adapter.is_err() {
            println!(
                "[ADAPTER] HighPerformance adapter not found. Attempting fallback/software adapter..."
            );
            adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: surface.as_ref(),
                force_fallback_adapter: true,
            }));
        }

        let adapter = adapter.unwrap();
        println!("[ADAPTER] {:?}", adapter.get_info());

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::FLOAT32_FILTERABLE,
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
        .expect("Failed to create device");

        Self::from_components(adapter, device, queue, surface, config, log)
    }

    fn from_components(
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        mut surface: Option<wgpu::Surface<'static>>,
        config: Config,
        log: Option<Box<dyn std::io::Write>>,
    ) -> Self {
        let surface_format = if let Some(surface) = surface.as_mut() {
            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_caps.formats[0]);
            println!("[ADAPTER] Surface format: {:?}", surface_format);

            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
                format: surface_format,
                width: config.width as u32,
                height: config.height as u32,
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

        let ssaa = SsaaPipeline::new(
            &device,
            surface_format,
            surface.is_some(),
            config.width,
            config.height,
            config.ssaa,
        );
        let compute = ComputePipeline::new(&device, surface_format, &ssaa, &config);
        let orbit = Orbit::new(&device);
        let palette = Palette::new(&device, &queue, &parse_palette(&config.palette));

        let (bytes_per_row, buffer_size) =
            output_buffer_bytes_per_row_and_size(config.width, config.height);
        let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let z = float_from_str(&config.zoom);
        let x = float_from_str(&config.x);
        let y = float_from_str(&config.y);

        Pipeline {
            surface,
            device,
            queue,
            config,
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
            //
            log,
        }
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.config.width, self.config.height)
    }

    pub fn total_pixels(&self) -> usize {
        let sf = self.ssaa.ssaa_factor();
        let (w, h) = self.dimensions();
        w * sf * h * sf
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

        let xexp = self.x.get_exponent().unwrap_or(0).unsigned_abs();
        let yexp = self.y.get_exponent().unwrap_or(0).unsigned_abs();
        let zexp = self.z.get_exponent().unwrap_or(0).unsigned_abs();
        let required = (64 + zexp + xexp.max(yexp)) as u64;

        if self.x.get_prec().is_some_and(|p| p < required) {
            self.x.set_prec(required);
        }
        if self.y.get_prec().is_some_and(|p| p < required) {
            self.y.set_prec(required);
        }
        if self.z.get_prec().is_some_and(|p| p < required) {
            self.z.set_prec(required);
        }

        result
    }

    pub fn read_config<R>(&mut self, f: impl FnOnce(&Config) -> R) -> R {
        f(&self.config)
    }

    pub fn write_config<R>(&mut self, f: impl FnOnce(&mut Config) -> R) -> R {
        let result = f(&mut self.config);
        self.updated_position = true;
        self.z = float_from_str(&self.config.zoom);
        self.x = float_from_str(&self.config.x);
        self.y = float_from_str(&self.config.y);
        self.palette = Palette::new(
            &self.device,
            &self.queue,
            &parse_palette(&self.config.palette),
        );
        result
    }

    /// Renders pixels with an iteration limit.
    ///
    /// Returns the remaining pixels to render.
    pub fn step_mandelbrot(&mut self, iterations: usize) -> u32 {
        let surface_texture = self
            .surface
            .as_ref()
            .map(|surface| surface.get_current_texture().unwrap());

        if self.finished() {
            if let Some(surface) = surface_texture {
                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                self.ssaa.render_pass(
                    &mut encoder,
                    Some(surface.texture.create_view(&Default::default())),
                );
                self.queue.submit([encoder.finish()]);
                surface.present();
            }

            return 0;
        }

        if self.updated_position {
            self.updated_position = false;
            self.orbit
                .compute_reference_orbit(&self.x, &self.y, &self.z, iterations);
            self.orbit.write_buffers(&self.queue, &self.z);
            self.compute
                .write_buffers(&self.queue, &self.config, &self.z, &self.palette);
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
            self.config.width,
            self.config.height,
        );
        self.ssaa.render_pass(
            &mut encoder,
            surface_texture
                .as_ref()
                .map(|surface_texture| surface_texture.texture.create_view(&Default::default())),
        );
        let remaining = self
            .compute
            .remaining_pixels(&self.device, &self.queue, encoder);
        self.finished_render = remaining == 0;
        if let Some(surface) = surface_texture {
            surface.present();
        }
        remaining
    }

    /// Renders pixels with an iteration limit.
    ///
    /// Continues to draw whether or not pixels are remaining.
    pub fn force_step_mandelbrot(&mut self, iterations: usize) {
        if self.updated_position {
            self.updated_position = false;
            self.orbit
                .compute_reference_orbit(&self.x, &self.y, &self.z, iterations);
            self.orbit.write_buffers(&self.queue, &self.z);
            self.compute
                .write_buffers(&self.queue, &self.config, &self.z, &self.palette);
        }

        let surface_texture = self
            .surface
            .as_ref()
            .map(|surface| surface.get_current_texture().unwrap());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.compute.compute_mandelbrot(
            &self.queue,
            &mut encoder,
            &self.orbit,
            &self.palette,
            &self.ssaa,
            self.config.width,
            self.config.height,
        );
        self.ssaa.render_pass(
            &mut encoder,
            surface_texture
                .as_ref()
                .map(|surface_texture| surface_texture.texture.create_view(&Default::default())),
        );
        self.queue.submit([encoder.finish()]);
        if let Some(surface) = surface_texture {
            surface.present();
        }
    }

    /// [`Pipeline::step_mandelbrot`] without rendering into the offscreen buffer.
    pub fn step_mandelbrot_headless(&mut self) -> u32 {
        if self.finished() {
            return 0;
        }

        if self.updated_position {
            self.updated_position = false;
            self.orbit
                .compute_reference_orbit(&self.x, &self.y, &self.z, self.config.iterations);
            self.orbit.write_buffers(&self.queue, &self.z);
            self.compute
                .write_buffers(&self.queue, &self.config, &self.z, &self.palette);
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
            self.config.width,
            self.config.height,
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
        self.ssaa.render_pass(
            &mut encoder,
            self.surface.as_ref().map(|surface| {
                surface
                    .get_current_texture()
                    .unwrap()
                    .texture
                    .create_view(&Default::default())
            }),
        );
        self.queue.submit([encoder.finish()]);
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
                texture: self.ssaa.output_texture().unwrap(),
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
                width: self.config.width as u32,
                height: self.config.height as u32,
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
        let mut result = Vec::with_capacity(self.config.width * self.config.height * 4);
        for chunk in padded_data.chunks(self.bytes_per_row) {
            result.extend_from_slice(&chunk[..self.config.width * 4]);
        }
        drop(padded_data);
        self.output_buffer.unmap();

        result
    }

    /// Write the current position and iterations for `frame`.
    pub fn log(&mut self, frame: usize) -> std::io::Result<()> {
        if let Some(log) = &mut self.log {
            log.write_all(format!("[FRAME] {frame}\n").as_bytes())?;
            log.write_all(format!("x = \"{}\"\n", self.x).as_bytes())?;
            log.write_all(format!("y = \"{}\"\n", self.y).as_bytes())?;
            log.write_all(format!("zoom = \"{}\"\n", self.z).as_bytes())?;
            log.write_all(format!("iterations = {}\n\n", self.config.iterations).as_bytes())?;
            log.flush()?;
        }
        Ok(())
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
