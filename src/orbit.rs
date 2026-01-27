use crate::{byte_slice, precision};
use rug::{Assign, Float, float::Round, ops::AddAssignRound};

#[repr(C)]
struct OrbitUniform {
    points: u32,
    polylim: u32,
    poly_scale_exponent: i32,
    coefficients: [f32; 6],
}

#[repr(C)]
struct RefPoint {
    x: f32,
    y: f32,
    s: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct WFloat {
    m: f32,
    e: i32,
}

impl WFloat {
    const ZERO: Self = Self { m: 0.0, e: 0 };
}

/// Reference orbit points and series approximation coefficients.
pub struct Orbit {
    pub bind_group: wgpu::BindGroup,
    point_buffer: wgpu::Buffer,
    uniform: wgpu::Buffer,
    points: Vec<RefPoint>,
    coefficients: [WFloat; 6],
    polylim: usize,
}

impl Orbit {
    pub fn new(device: &wgpu::Device) -> Self {
        // TODO: This is not true!
        const MAX_ITERATIONS: usize = 1_000_000;
        let size = std::mem::size_of::<RefPoint>() * MAX_ITERATIONS;

        let point_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: std::mem::size_of::<OrbitUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &Self::bind_group_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: point_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            bind_group,
            point_buffer,
            uniform,
            points: Vec::with_capacity(size),
            coefficients: [WFloat::ZERO; 6],
            polylim: 0,
        }
    }

    pub fn bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    /// Must be called before [`Orbit::write_buffers`].
    // Algorithm ported from JS: https://github.com/HastingsGreer/mandeljs/blob/7bb12c6ee2214e4eea82a30498de85823b3be474/main.js#L410
    pub fn compute_reference_orbit(
        &mut self,
        x0: &Float,
        y0: &Float,
        z: &Float,
        iterations: usize,
    ) {
        self.points.clear();
        self.polylim = 0;

        let prec = precision(z);
        let mut x = Float::with_val(prec, 0.0);
        let mut y = Float::with_val(prec, 0.0);
        let mut txx = Float::with_val(prec, 0.0);
        let mut txy = Float::with_val(prec, 0.0);
        let mut tyy = Float::with_val(prec, 0.0);

        let mut bx = WFloat::ZERO;
        let mut by = WFloat::ZERO;
        let mut cx = WFloat::ZERO;
        let mut cy = WFloat::ZERO;
        let mut dx = WFloat::ZERO;
        let mut dy = WFloat::ZERO;

        let mut not_failed = true;
        for i in 0..iterations {
            // check if x and y are both representable as 32 bit floats
            let x_exponent = x.get_exp().unwrap_or(0);
            let y_exponent = y.get_exp().unwrap_or(0);

            let mut scale_exponent = x_exponent.max(y_exponent);
            if scale_exponent < -10000 {
                scale_exponent = 0;
            }

            let (xm, _) = x.to_f32_exp();
            let (ym, _) = y.to_f32_exp();

            let fx = xm / 2f32.powi(scale_exponent - x_exponent);
            let fy = ym / 2f32.powi(scale_exponent - y_exponent);

            self.points.push(RefPoint {
                x: fx,
                y: fy,
                s: scale_exponent,
            });

            let fx = WFloat {
                m: fx,
                e: scale_exponent,
            };
            let fy = WFloat {
                m: fy,
                e: scale_exponent,
            };

            txx.assign(&x * &x);
            txy.assign(&x * &y);
            tyy.assign(&y * &y);

            x.assign(&txx - &tyy);
            x.add_assign_round(x0, Round::Nearest);
            y.assign(&txy + &txy);
            y.add_assign_round(y0, Round::Nearest);

            let one = WFloat { m: 1.0, e: 0 };
            let two = WFloat { m: 2.0, e: 0 };

            let tbx = add(mul(two, sub(mul(fx, bx), mul(fy, by))), one);
            let tby = mul(two, add(mul(fx, by), mul(fy, bx)));
            let tcx = sub(
                add(mul(two, sub(mul(fx, cx), mul(fy, cy))), mul(bx, bx)),
                mul(by, by),
            );
            let tcy = add(
                mul(two, add(mul(fx, cy), mul(fy, cx))),
                mul(mul(two, bx), by),
            );
            let tdx = mul(
                two,
                add(sub(mul(fx, dx), mul(fy, dy)), sub(mul(cx, bx), mul(cy, by))),
            );
            let tdy = mul(
                two,
                add(add(add(mul(fx, dy), mul(fy, dx)), mul(cx, by)), mul(cy, bx)),
            );

            let (xm, xe) = x.to_f32_exp();
            let fx = WFloat { m: xm, e: xe };

            let (ym, ye) = y.to_f32_exp();
            let fy = WFloat { m: ym, e: ye };

            if i == 0
                || gt(
                    maxabs(tcx, tcy),
                    mul(
                        WFloat {
                            m: 1000.0,
                            e: z.get_exp().unwrap_or(0) + 25,
                        },
                        maxabs(tdx, tdy),
                    ),
                )
            {
                if not_failed {
                    self.polylim = i;
                    self.coefficients = [bx, by, cx, cy, dx, dy];
                    bx = tbx;
                    by = tby;
                    cx = tcx;
                    cy = tcy;
                    dx = tdx;
                    dy = tdy;
                }
            } else {
                not_failed = false;
            }

            if gt(add(mul(fx, fx), mul(fy, fy)), WFloat { m: 400.0, e: 0 }) {
                break;
            }
        }
    }

    /// Uploads point and approximation uniform buffers.
    ///
    /// Call [`Orbit::compute_reference_orbit`] first.
    pub fn write_buffers(&self, queue: &wgpu::Queue, z: &Float) {
        let (r, rexp) = z.to_f32_exp();
        let r = WFloat { m: r, e: rexp };

        let poly_scape_exp = mul(
            WFloat { m: 1.0, e: 0 },
            maxabs(self.coefficients[0], self.coefficients[1]),
        );

        let poly_scale = WFloat {
            m: 1.0,
            e: -poly_scape_exp.e,
        };

        let poly_scaled = [
            mul(poly_scale, self.coefficients[0]),
            mul(poly_scale, self.coefficients[1]),
            mul(poly_scale, mul(r, self.coefficients[2])),
            mul(poly_scale, mul(r, self.coefficients[3])),
            mul(poly_scale, mul(r, mul(r, self.coefficients[4]))),
            mul(poly_scale, mul(r, mul(r, self.coefficients[5]))),
        ]
        .map(|d| d.m * 2f32.powi(d.e));

        let uniform = OrbitUniform {
            points: self.points.len() as u32,
            polylim: self.polylim as u32,
            poly_scale_exponent: poly_scape_exp.e,
            coefficients: poly_scaled,
        };

        queue.write_buffer(&self.uniform, 0, byte_slice(&[uniform]));
        queue.write_buffer(&self.point_buffer, 0, byte_slice(&self.points));
    }
}

fn split(a: WFloat, b: WFloat) -> (f32, f32, i32) {
    let ret_e = a.e.max(b.e);
    let mut am = a.m;
    let mut bm = b.m;
    if ret_e > a.e {
        am *= 2f32.powi(a.e - ret_e);
    } else {
        bm *= 2f32.powi(b.e - ret_e);
    }
    (am, bm, ret_e)
}

fn add(a: WFloat, b: WFloat) -> WFloat {
    let (am, bm, ret_e) = split(a, b);
    WFloat {
        m: am + bm,
        e: ret_e,
    }
}

fn sub(a: WFloat, b: WFloat) -> WFloat {
    let (am, bm, ret_e) = split(a, b);
    WFloat {
        m: am - bm,
        e: ret_e,
    }
}

fn mul(a: WFloat, b: WFloat) -> WFloat {
    let mut m = a.m * b.m;
    let mut e = a.e + b.e;
    if m != 0.0 {
        let logm = m.abs().log2().round() as i32;
        m /= 2f32.powi(logm);
        e += logm;
    }
    WFloat { m, e }
}

fn maxabs(a: WFloat, b: WFloat) -> WFloat {
    let (am, bm, ret_e) = split(a, b);
    WFloat {
        m: am.abs().max(bm.abs()),
        e: ret_e,
    }
}

fn gt(a: WFloat, b: WFloat) -> bool {
    let (am, bm, _) = split(a, b);
    am > bm
}
