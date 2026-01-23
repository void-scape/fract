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
    let py = (1.0 - in.uv.y) * f32(args.height);

	let dx0 = args.sdx + px * args.xstep;
	let dy0 = args.sdy + py * args.ystep;

	// Compute the delta of (x0, y0) with respect to the
	// reference orbit.
	var dx = dx0;
	var dy = dy0;
	var iteration = 0u;
	var ref_iteration = 0u;

	// If there are coefficients present, approximate the position
	// of (dx, dy) at iteration `approx_iteration`.
	if args.approx_iteration > 0 {
		// D = Ad + Bd^2 + Cd^3
		let d = dx0;
		let di = dy0;

		let d2 = d * d - di * di;
		let d2i = di * d + d * di;

		let d3 = d2 * d - d2i * di;
		let d3i = d2 * di + d2i * d;

		let ad = args.ax * d - args.ay * di;
		let adi = args.ay * d + args.ax * di;

		let bd2 = args.bx * d2 - args.by * d2i;
		let bd2i = args.by * d2 + args.bx * d2i;

		let cd3 = args.cxx * d3 - args.cyy * d3i;
		let cd3i = args.cyy * d3 + args.cxx * d3i;

		dx = ad + bd2 + cd3;
		dy = adi + bd2i + cd3i;

		iteration = args.approx_iteration;
		ref_iteration = args.approx_iteration;
	}

	while iteration < args.max_iteration {
		var ax = orbit[ref_iteration * 2];
		var ay = orbit[ref_iteration * 2 + 1];
		ax *= 2.0;
		ay *= 2.0;

		// ad = a * d
		let adx = ax * dx - ay * dy;
		let ady = ax * dy + ay * dx;

		// a = a * d + d * d
		ax = adx + dx * dx - dy * dy;
		ay = ady + dx * dy + dy * dx;

		// d = a * d + d * d + d0
		dx = ax + dx0;
		dy = ay + dy0;

		ref_iteration += 1;

		// The full value of (x0, y0) is reconstructed from
		// the reference orbit and checked for escape time.
		let x = orbit[ref_iteration * 2];
		let y = orbit[ref_iteration * 2 + 1];
		let zmag = (dx + x) * (dx + x) + (dy + y) * (dy + y);
		let dmag = dx * dx + dy * dy;

		if zmag > 10000.0 {
			break;
		} else if zmag < dmag || ref_iteration == args.orbit_len - 1 {
			dx += x;
			dy += y;
			ref_iteration = 0;
		}

		iteration += 1;
	}
    
	let x = orbit[ref_iteration * 2];
	let y = orbit[ref_iteration * 2 + 1];
	return iteration_to_rgb(iteration, x + dx, y + dy);
}
