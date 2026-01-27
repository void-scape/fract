struct MandelbrotUniform {
    iterations: u32,
    zm: f32, ze: i32,
	width: f32, height: f32,
}

struct OrbitUniform {
    points: u32,
    polylim: u32,
	poly_scale_exponent: i32,
	a: f32, b: f32,
	c: f32, d: f32,
	e: f32, f: f32,
}

struct RefPoint {
	x: f32, y: f32, e: i32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) delta: vec2<f32>,
}

@group(0) @binding(0) var<uniform> args: MandelbrotUniform;
@group(1) @binding(0) var<uniform> orbit: OrbitUniform;
@group(1) @binding(1) var<storage, read> points: array<RefPoint>;
@group(2) @binding(0) var palette: texture_2d<f32>;
@group(2) @binding(1) var palette_sampler: sampler;
override SWAP_CHANNELS: bool = false;

@vertex
fn vs_main(@builtin(vertex_index) id: u32) -> VertexOutput {
    var out: VertexOutput;
    let aspect = args.width / args.height;
    out.delta = vec2(f32((id << 1u) & 2u), f32(id & 2u));
    out.clip_position = vec4(out.delta * 2.0 + vec2(-1.0, -1.0), 0.0, 1.0);
	out.delta = out.delta * vec2(2.0) - vec2(1.0);
	out.delta.x *= aspect;
	out.delta *= args.zm * 2.0;
    return out;
}

// I am not going to pretend to understand this code: 
// https://github.com/HastingsGreer/mandeljs/blob/7bb12c6ee2214e4eea82a30498de85823b3be474/main.js#L198
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
	var q = args.ze - 1;
	let cq = q;
	q = q + orbit.poly_scale_exponent;
	var S = pow(2.0, f32(q));
	var dcx = in.delta.x;
	var dcy = in.delta.y;

	// dx + dyi = (p0 + p1 i) * (dcx, dcy) + (p2 + p3i) * (dcx + dcy * i) * (dcx + dcy * i)
	let sqrx = dcx * dcx - dcy * dcy;
	let sqry = 2.0 * dcx * dcy;
	
	let cux = dcx * sqrx - dcy * sqry;
	let cuy = dcx * sqry + dcy * sqrx;
	var dx = orbit.a * dcx - orbit.b * dcy + orbit.c * sqrx - orbit.d * sqry + orbit.e * cux - orbit.f * cuy;
	var dy = orbit.a * dcy + orbit.b * dcx + orbit.c * sqry + orbit.d * sqrx + orbit.e * cuy + orbit.f * cux;

	var k = i32(orbit.polylim);
	var j = k;

	var x = points[k].x;
	var y = points[k].y;

	for (var i = k; i < i32(args.iterations); i++) {
		j += 1;
		k += 1;

		let os = points[k - 1].e;
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

		x = points[k].x;
		y = points[k].y;
		let fx = x * pow(2.0, f32(points[k].e)) + S * dx;
		let fy = y * pow(2.0, f32(points[k].e)) + S * dy;

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
				|| (k >= (i32(orbit.points) - 1))
		) {
			dx = fx;
			dy = fy;
			q = 0;
			S = pow(2.0, f32(q));
			dcx = in.delta.x * pow(2.0, f32(-q + cq));
			dcy = in.delta.y * pow(2.0, f32(-q + cq));
			k = 0;
			x = points[0].x;
			y = points[0].y;
		}
	}

	x = points[k].x;
	y = points[k].y;
	let fx = x * pow(2.0, f32(points[k].e)) + S * dx;
	let fy = y * pow(2.0, f32(points[k].e)) + S * dy;
 	return iteration_to_rgb(u32(j), fx, fy);
}

fn iteration_to_rgb(iteration: u32, x: f32, y: f32) -> vec4<f32> {
    if (iteration == args.iterations) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }

	// https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring
    let zn = dot(vec2(x, y), vec2(x, y));
    let nu = log2(log2(zn) * 0.5);
    let iter = f32(iteration) + 1.0 - nu;

	let val = log2(iter) / 8.0; 
    let uv = vec2(val, 0.5);
	// let uv = vec2(iter / 24.0, 0.5);

	let rgb = textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
    return vec4(select(rgb.bgr, rgb, SWAP_CHANNELS), 1.0);
}
