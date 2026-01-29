struct MandelbrotUniform {
    iterations: i32,
    zm: f32, ze: i32,
	batch_iter: i32,
    palette_len: f32,
	color_scale: f32,
	color_mode: i32,
}

struct OrbitUniform {
    points: i32,
    polylim: i32,
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
    j: i32, k: i32,
    q: i32, finished: u32,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: MandelbrotUniform;
@group(0) @binding(2) var<storage, read_write> states: array<OrbitState>;
@group(0) @binding(3) var<storage, read_write> remaining: atomic<u32>;

@group(1) @binding(0) var<uniform> orbit: OrbitUniform;
@group(1) @binding(1) var<storage, read> points: array<RefPoint>;

@group(2) @binding(0) var palette: texture_2d<f32>;
@group(2) @binding(1) var palette_sampler: sampler;
override SWAP_CHANNELS: bool = false;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>, @builtin(local_invocation_index) local_id: u32) {
    let sz = textureDimensions(output);
    if (id.x >= sz.x || id.y >= sz.y) { return; }
    let aspect = f32(sz.x) / f32(sz.y);
    var uv = vec2<f32>(f32(id.x), f32(sz.y - id.y)) / vec2<f32>(sz.xy) * 2.0 - 1.0;
	uv.x *= aspect;
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
		return color(state);
    }

	var dx = state.dx;
    var dy = state.dy;
    var j  = state.j;
    var k  = state.k;
    var q  = state.q;
    let cq = args.ze - 1;

	if (j == 0) {
        q = cq + orbit.poly_scale_exponent;
        k = orbit.polylim;
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

	let batch_limit = j + args.batch_iter;
	while (j < batch_limit && j < args.iterations) {
		j += 1;
		k += 1;

		let os = points[k - 1].e;
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

		x = points[k].x;
		y = points[k].y;
		let fx = x * exp2(f32(points[k].e)) + S * dx;
		let fy = y * exp2(f32(points[k].e)) + S * dy;

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
				|| (k >= (orbit.points - 1))
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

	if (j >= args.iterations) {
        state.finished = 1u;
    }

	state.dx = dx;
    state.dy = dy;
    state.j = j;
    state.k = k;
    state.q = q;
    states[state_index] = state;

	return color(state);
}

fn color(state: OrbitState) -> vec4<f32> {
    if (state.j == args.iterations) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }

	if args.color_mode == 0 {
		let denom = args.palette_len * args.color_scale;
		return sample(f32(state.j) / denom);
	}
	if args.color_mode == 1 {
		return wave(f32(state.j));
	}

	let x = points[state.k].x;
	let y = points[state.k].y;
	let S = exp2(f32(state.q));
	let fx = x * exp2(f32(points[state.k].e)) + S * state.dx;
	let fy = y * exp2(f32(points[state.k].e)) + S * state.dy;

	// https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring
    let zn = dot(vec2(fx, fy), vec2(fx, fy));
    let nu = log2(log2(zn) * 0.5);
    let iteration = f32(state.j) + 1.0 - nu;

	if args.color_mode == 2 {
		let denom = args.palette_len * args.color_scale;
		return sample(iteration / denom);
	}
	if args.color_mode == 3 {
		return wave(iteration);
	}

	// this should never trigger, but if it does it will be obvious
	return vec4(1.0, 0.0, 1.0, 1.0);
}

fn wave(iteration: f32) -> vec4<f32> {
	var count = iteration;
	let period = 64.0 * args.color_scale;
	let mo = 0.0;

	var rp = 1.0;
	var gp = 1.0;
	var bp = 1.0;

	let uv = vec2(count, 0.5);
	let palette_rgb = textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;

	// rp = rp + palette_rgb.r * args.color_scale;
	// gp = gp + palette_rgb.g * args.color_scale;
	// bp = bp + palette_rgb.b * args.color_scale;

	var ro = -0.47;
	var go = 0.0;
	var bo = 0.7;

	let tau = 2.0 * 3.14159265;
	// ro = (ro + palette_rgb.r) % tau;
	// go = (go + palette_rgb.g) % tau;
	// bo = (bo + palette_rgb.b) % tau;

	let ang = count * tau / period;
	var r = 0.5 + 0.5 * sin(ang * rp + ro + mo);
    var g = 0.5 + 0.5 * sin(ang * gp + go + mo);
    var b = 0.5 + 0.5 * sin(ang * bp + bo + mo);

	// r = (r + palette_rgb.r) % 1.0;
	// g = (g + palette_rgb.g) % 1.0;
	// b = (b + palette_rgb.b) % 1.0;

	let rgb = vec3(r, g, b);
    return vec4(select(rgb.bgr, rgb, SWAP_CHANNELS), 1.0);
}

fn sample(x: f32) -> vec4<f32> {
	let uv = vec2(x, 0.5);
	let rgb = textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
    return vec4(select(rgb.bgr, rgb, SWAP_CHANNELS), 1.0);
}
