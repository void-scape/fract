struct MandelbrotUniform {
    iterations: u32,
    zm: f32, ze: i32,
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

struct OrbitState {
    dx: f32, dy: f32,
    j: u32, k: i32,
    q: i32, finished: u32,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: MandelbrotUniform;
@group(0) @binding(2) var<storage, read_write> states: array<OrbitState>;
@group(0) @binding(3) var<storage, read_write> remaining: atomic<u32>;

@group(1) @binding(0) var<uniform> orbit: OrbitUniform;
@group(1) @binding(1) var<storage, read> points: array<RefPoint>;

override SWAP_CHANNELS: bool = false;
@group(2) @binding(0) var palette: texture_2d<f32>;
@group(2) @binding(1) var palette_sampler: sampler;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) local_id: u32) {
    let sz = textureDimensions(output);
    if (id.x >= sz.x || id.y >= sz.y) { return; }
    let aspect = f32(sz.x) / f32(sz.y);
    var uv = vec2<f32>(f32(id.x) * aspect, f32(sz.y - id.y)) / vec2<f32>(sz.xy) * 2.0 - 1.0;
	let state_index = id.y * sz.x + id.x;
    textureStore(output, id.xy, mandelbrot(state_index, uv * args.zm * 2.0));
	if (states[state_index].finished == 0u) {
		atomicAdd(&remaining, 1u);
    }
}

// I am not going to pretend to understand this code: 
// https://github.com/HastingsGreer/mandeljs/blob/7bb12c6ee2214e4eea82a30498de85823b3be474/main.js#L198
fn mandelbrot(state_index: u32, delta: vec2<f32>) -> vec4<f32> {
	var state = states[state_index];

	if (state.finished == 1u) {
        let x = points[state.k].x;
        let y = points[state.k].y;
        let S = exp2(f32(state.q));
        let fx = x * exp2(f32(points[state.k].e)) + S * state.dx;
        let fy = y * exp2(f32(points[state.k].e)) + S * state.dy;
        return iteration_to_rgb(state.j, fx, fy);
    }

	var dx = state.dx;
    var dy = state.dy;
    var j  = i32(state.j);
    var k  = state.k;
    var q  = state.q;
    let cq = args.ze - 1;

	if (j == 0) {
        q = cq + orbit.poly_scale_exponent;
        k = i32(orbit.polylim);
        j = k;
        
        let dcx_init = delta.x;
        let dcy_init = delta.y;
        let sqrx = dcx_init * dcx_init - dcy_init * dcy_init;
        let sqry = 2.0 * dcx_init * dcy_init;
        let cux = dcx_init * sqrx - dcy_init * sqry;
        let cuy = dcx_init * sqry + dcy_init * sqrx;
        
        dx = orbit.a * dcx_init - orbit.b * dcy_init + orbit.c * sqrx - orbit.d * sqry + orbit.e * cux - orbit.f * cuy;
        dy = orbit.a * dcy_init + orbit.b * dcx_init + orbit.c * sqry + orbit.d * sqrx + orbit.e * cuy + orbit.f * cux;
    }

	var S = exp2(f32(q));
    var dcx = delta.x * exp2(f32(-q + cq));
    var dcy = delta.y * exp2(f32(-q + cq));

	var x = points[k].x;
	var y = points[k].y;

	var x0 = points[0].x;
	var y0 = points[0].y;
	var prev_pt = points[k];
	var current_pt = points[k + 1];

	let batch_limit = j + 250;
	while (j < batch_limit && j < i32(args.iterations)) {
		j += 1;
		k += 1;

		let os = prev_pt.e;
		dcx = delta.x * exp2(f32(-q + cq - os));
		dcy = delta.y * exp2(f32(-q + cq - os));
		var unS = exp2(f32(q) - f32(os));

		if (abs(unS) > 3.4028235e34) {
			unS = 0.0;
		}

		let tx = 2.0 * x * dx - 2.0 * y * dy + unS * dx * dx - unS * dy * dy + dcx;
		dy = 2.0 * x * dy + 2.0 * y * dx + unS * 2.0 * dx * dy + dcy;
		dx = tx;

		q = q + os;
		S = exp2(f32(q));

		current_pt = points[k];
		x = current_pt.x;
		y = current_pt.y;
		let fx = x * exp2(f32(current_pt.e)) + S * dx;
		let fy = y * exp2(f32(current_pt.e)) + S * dy;

		prev_pt = current_pt;

		if (fx * fx + fy * fy > 10000.0) {
			state.finished = 1u;
			break;
		}

		if (dx * dx + dy * dy > 1000000.0) {
			dx = dx / 2.0;
			dy = dy / 2.0;
			q = q + 1;
			S = exp2(f32(q));
			dcx = delta.x * exp2(f32(-q + cq));
			dcy = delta.y * exp2(f32(-q + cq));
		}

		if (
			(fx * fx + fy * fy < S * S * dx * dx + S * S * dy * dy) 
				|| (k >= (i32(orbit.points) - 1))
		) {
			dx = fx;
			dy = fy;
			q = 0;
			S = exp2(f32(q));
			dcx = delta.x * exp2(f32(-q + cq));
			dcy = delta.y * exp2(f32(-q + cq));
			k = 0;
			x = x0;
			y = y0;
		}
	}

	if (j >= i32(args.iterations)) {
        state.finished = 1u;
    }

	state.dx = dx;
    state.dy = dy;
    state.j = u32(j);
    state.k = k;
    state.q = q;
    states[state_index] = state;

	x = points[k].x;
	y = points[k].y;
	let fx = x * exp2(f32(points[k].e)) + S * dx;
	let fy = y * exp2(f32(points[k].e)) + S * dy;
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
