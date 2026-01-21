struct MandelbrotUniform {
    width: u32,
    height: u32,
    max_iteration: u32,
    zoom: f32,
    cx: f32,
    cy: f32,
};

@group(0) @binding(0) var<uniform> args: MandelbrotUniform;
@group(0) @binding(1) var output_tex: texture_storage_2d<rgba8unorm, write>;

fn hsv2rgb(c: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
    let p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
    return c.z * mix(K.xxx, clamp(p - K.xxx, vec3(0.0), vec3(1.0)), c.y);
}

@compute @workgroup_size(8, 8)
fn mandelbrot(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= args.width || id.y >= args.height) {
        return;
    }

    let w = f32(args.width);
    let h = f32(args.height);
    let px = f32(id.x);
    let py = f32(id.y);

    let MANDELBROT_XRANGE = 2.47;
    let MANDELBROT_YRANGE = 2.24;

    let x0 = ((px / w * MANDELBROT_XRANGE) - 2.00) * args.zoom + args.cx;
    let y0 = ((py / h * MANDELBROT_YRANGE) - 1.12) * args.zoom + args.cy;

    var x: f32 = 0.0;
    var y: f32 = 0.0;
    var iteration: u32 = 0;

    while (iteration < args.max_iteration) {
        let x2 = x * x;
        let y2 = y * y;

        if (x2 + y2 > 4.0) {
            break;
        }

        y = (2.0 * x * y) + y0;
        x = x2 - y2 + x0;
        iteration = iteration + 1;
    }

    let l = f32(iteration) / f32(args.max_iteration);
    let rgb = hsv2rgb(vec3<f32>(l, 1.0, 1.0));
    textureStore(output_tex, id.xy, vec4<f32>(rgb, 1.0));
}
