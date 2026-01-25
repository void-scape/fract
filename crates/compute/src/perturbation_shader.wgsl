struct MandelbrotUniform {
    width: u32,
    height: u32,
    max_iteration: u32,
	orbit_len: u32,
	zoom: f32,
	cx: f32,
	cy: f32,
	q: i32,
}

struct OrbitDelta {
	dx: f32,
	dy: f32,
	exponent: i32,
	_pad: u32,
}

@group(0) @binding(0) var<uniform> args: MandelbrotUniform;
@group(0) @binding(1) var<storage, read> orbit: array<OrbitDelta>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) delta: vec2<f32>,
}

override SWAP_CHANNELS: bool = false;

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
	var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0) - 0.5,
        vec2<f32>(2.0, 0.0) - 0.5,
        vec2<f32>(0.0, 2.0) - 0.5,
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[id], 0.0, 1.0);
    out.delta = vec2(uv[id].x * f32(args.width) / f32(args.height), -uv[id].y);
    return out;
}

// I am not going to pretend to understand this code: 
// https://github.com/HastingsGreer/mandeljs/blob/7bb12c6ee2214e4eea82a30498de85823b3be474/main.js#L198
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	var q = args.q - 1;
	let cq = q;
	var S = pow(2.0, f32(q));
	var dcx = in.delta.x;
	var dcy = in.delta.y;

	var dx = 0.0;
	var dy = 0.0;

	var j = 0;
	var k = 0;

	var x = orbit[k].dx;
	var y = orbit[k].dy;

	for (var i = k; i < i32(args.max_iteration); i++) {
		j += 1;
		k += 1;

		let os = orbit[k - 1].exponent;
		dcx = in.delta.x * pow(2.0, f32(-q + cq - os));
		dcy = in.delta.y * pow(2.0, f32(-q + cq - os));
		var unS = pow(2.0, f32(q) - f32(os));

		if (abs(unS) > 3.4028235e34) {
			unS = 0.0;
		}

		let tx = 2.0 * x * dx - 2.0 * y * dy + unS * dx * dx - unS * dy * dy + dcx;
		dy = 2.0 * x * dy + 2.0 * y * dx + unS * 2.0 * dx * dy + dcy;
		dx = tx;

		q = q + os;
		S = pow(2.0, f32(q));

		x = orbit[k].dx;
		y = orbit[k].dy;
		let fx = x * pow(2.0, f32(orbit[k].exponent)) + S * dx;
		let fy = y * pow(2.0, f32(orbit[k].exponent)) + S * dy;
		if (fx * fx + fy * fy > 10000.0) {
			break;
		}

		if (dx * dx + dy * dy > 1000000.0) {
			dx = dx / 2.0;
			dy = dy / 2.0;
			q = q + 1;
			S = pow(2.0, f32(q));
			dcx = in.delta.x * pow(2.0, f32(-q + cq));
			dcy = in.delta.y * pow(2.0, f32(-q + cq));
		}

		if (
			(fx * fx + fy * fy < S * S * dx * dx + S * S * dy * dy) 
				|| (k >= (i32(args.orbit_len) - 1))
		) {
			dx = fx;
			dy = fy;
			q = 0;
			S = pow(2.0, f32(q));
			dcx = in.delta.x * pow(2.0, f32(-q + cq));
			dcy = in.delta.y * pow(2.0, f32(-q + cq));
			k = 0;
			x = orbit[0].dx;
			y = orbit[0].dy;
		}
	}

	x = orbit[k].dx;
	y = orbit[k].dy;
	let fx = x * pow(2.0, f32(orbit[k].exponent)) + S * dx;
	let fy = y * pow(2.0, f32(orbit[k].exponent)) + S * dy;
 	return iteration_to_rgb(u32(j), fx, fy);
}
