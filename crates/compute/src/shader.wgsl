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
}

@group(0) @binding(0) var<uniform> args: MandelbrotUniform;
@group(0) @binding(1) var<storage, read> orbit: array<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

override SWAP_CHANNELS: bool = false;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 2.0)
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
    out.uv = uv[in_vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let px = in.uv.x * f32(args.width);
    let py = in.uv.y * f32(args.height);

	let aspect = f32(args.width) / f32(args.height);
	let x0 = (px / f32(args.width) * 2.0 - 1.0) * args.zoom * aspect + args.cx;
	let y0 = (py / f32(args.height) * 2.0 - 1.0) * args.zoom - args.cy;

    var x: f32 = 0.0;
    var y: f32 = 0.0;
    var iteration: u32 = 0;

    while (iteration < args.max_iteration) {
        let x2 = x * x;
        let y2 = y * y;

        if (x2 + y2 > 10000.0) {
            break;
        }

        y = (2.0 * x * y) + y0;
        x = x2 - y2 + x0;
        iteration = iteration + 1;
    }

	return iteration_to_rgb(iteration, x, y);
}
