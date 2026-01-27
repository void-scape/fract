use crate::byte_slice;
use tint::{Color, LinearRgb, Sbgr};

pub fn parse_palette(palette: &str) -> Vec<Sbgr> {
    match palette {
        "classic" => classic().to_vec(),
        "lava" => lava().to_vec(),
        "ocean" => ocean().to_vec(),
        _ => {
            println!("Unknown palette: {}", palette);
            std::process::exit(1);
        }
    }
}

// https://stackoverflow.com/a/16505538
pub fn classic() -> [Sbgr; 16] {
    [
        LinearRgb::from_rgb(66.0 / 255.0, 30.0 / 255.0, 15.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(25.0 / 255.0, 7.0 / 255.0, 26.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(9.0 / 255.0, 1.0 / 255.0, 47.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(4.0 / 255.0, 4.0 / 255.0, 73.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 7.0 / 255.0, 100.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(12.0 / 255.0, 44.0 / 255.0, 138.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(24.0 / 255.0, 82.0 / 255.0, 177.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(57.0 / 255.0, 125.0 / 255.0, 209.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(134.0 / 255.0, 181.0 / 255.0, 229.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(211.0 / 255.0, 236.0 / 255.0, 248.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(241.0 / 255.0, 233.0 / 255.0, 191.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(248.0 / 255.0, 201.0 / 255.0, 95.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 170.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(204.0 / 255.0, 128.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(153.0 / 255.0, 87.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(106.0 / 255.0, 52.0 / 255.0, 3.0 / 255.0).to_sbgr(),
    ]
}

// https://github.com/bertbaron/mandelbrot/blob/38b88b0bf5dcbe5cb214637964515197a56e124d/palette.js#L125
pub fn lava() -> [Sbgr; 24] {
    [
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(10.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(20.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(40.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(80.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(160.0 / 255.0, 10.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(200.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(240.0 / 255.0, 90.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 160.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 220.0 / 255.0, 10.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 80.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 160.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 160.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 80.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 220.0 / 255.0, 10.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 160.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(240.0 / 255.0, 90.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(200.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(160.0 / 255.0, 10.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(80.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(40.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(20.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(10.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
    ]
}

// https://github.com/bertbaron/mandelbrot/blob/38b88b0bf5dcbe5cb214637964515197a56e124d/palette.js#L148
pub fn ocean() -> [Sbgr; 18] {
    [
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 51.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 102.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 153.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 51.0 / 255.0, 102.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 102.0 / 255.0, 204.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(51.0 / 255.0, 153.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(102.0 / 255.0, 178.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(153.0 / 255.0, 204.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(204.0 / 255.0, 229.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(204.0 / 255.0, 229.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(153.0 / 255.0, 204.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(102.0 / 255.0, 178.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(51.0 / 255.0, 153.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 102.0 / 255.0, 204.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 51.0 / 255.0, 102.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 153.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 102.0 / 255.0).to_sbgr(),
    ]
}

/// Stores a palette in a texture.
pub struct Palette {
    pub bind_group: wgpu::BindGroup,
}

impl Palette {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, palette: &[Sbgr]) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
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
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self { bind_group }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }
}
