struct MandelbrotUniform {
    iterations: u32,
    zm: f32,
    ze: i32,
}

struct OrbitUniform {
    points: u32,
    polylim: u32,
	poly_scale_exponent: i32,
	a: f32,
	b: f32,
	c: f32,
	d: f32,
	e: f32,
	f: f32,
}

struct RefPoint {
	x: f32,
	y: f32,
	e: i32,
	_pad: u32,
}

@group(0) @binding(0) var output: texture_storage_2d<rgba32float, write>;
@group(0) @binding(1) var<uniform> args: MandelbrotUniform;

@group(1) @binding(0) var<uniform> orbit: OrbitUniform;
@group(1) @binding(1) var<storage, read> points: array<RefPoint>;

override SWAP_CHANNELS: bool = false;
@group(2) @binding(0) var palette: texture_2d<f32>;
@group(2) @binding(1) var palette_sampler: sampler;

@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let sz = textureDimensions(output);
    if (id.x >= sz.x || id.y >= sz.y) { return; }
    let aspect = f32(sz.x) / f32(sz.y);
    var uv = vec2<f32>(f32(id.x) * aspect, f32(sz.y - id.y)) / vec2<f32>(sz.xy) * 2.0 - 1.0;
    textureStore(output, id.xy, mandel(uv * args.zm * 2.0));
}

// I am not going to pretend to understand this code: 
// https://github.com/HastingsGreer/mandeljs/blob/7bb12c6ee2214e4eea82a30498de85823b3be474/main.js#L198
fn mandel(delta: vec2<f32>) -> vec4<f32> {
	var q = args.ze - 1;
	let cq = q;
	q = q + orbit.poly_scale_exponent;
	var S = pow(2.0, f32(q));
	var dcx = delta.x;
	var dcy = delta.y;

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

	var s1 = 0.0;
	var s2 = 0.0;
    let stripe_density = 10.0;

	for (var i = k; i < i32(args.iterations); i++) {
		j += 1;
		k += 1;

		let os = points[k - 1].e;
		dcx = delta.x * pow(2.0, f32(-q + cq - os));
		dcy = delta.y * pow(2.0, f32(-q + cq - os));
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

		s2 = s1;
		s1 += 0.5 * sin(stripe_density * atan2(fy, fx)) + 0.5;

		if (fx * fx + fy * fy > 10000.0) {
			break;
		}

		if (dx * dx + dy * dy > 1000000.0) {
			dx = dx / 2.0;
			dy = dy / 2.0;
			q = q + 1;
			S = pow(2.0, f32(q));
			dcx = delta.x * pow(2.0, f32(-q + cq));
			dcy = delta.y * pow(2.0, f32(-q + cq));
		}

		if (
			(fx * fx + fy * fy < S * S * dx * dx + S * S * dy * dy) 
				|| (k >= (i32(orbit.points) - 1))
		) {
			dx = fx;
			dy = fy;
			q = 0;
			S = pow(2.0, f32(q));
			dcx = delta.x * pow(2.0, f32(-q + cq));
			dcy = delta.y * pow(2.0, f32(-q + cq));
			k = 0;
			x = points[0].x;
			y = points[0].y;
		}
	}

	x = points[k].x;
	y = points[k].y;
	let fx = x * pow(2.0, f32(points[k].e)) + S * dx;
	let fy = y * pow(2.0, f32(points[k].e)) + S * dy;
	// return not_quite_stripes(u32(j), fx, fy, s1, s2);
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

	let uv = vec2(iter / 100.0, 0.5);
	let rgb = textureSampleLevel(palette, palette_sampler, uv, 0.0).rgb;
    return vec4(select(rgb.bgr, rgb, SWAP_CHANNELS), 1.0);
}

fn not_quite_stripes(iteration: u32, x: f32, y: f32, s1: f32, s2: f32) -> vec4<f32> {
    if (iteration == args.iterations) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }

    let zn = dot(vec2(x, y), vec2(x, y));
    let nu = log2(log2(zn) * 0.5);
    let iter = f32(iteration) + 1.0 - nu;

	let stripe = mix(s1, s2, nu);

    let avg_stripe = stripe / iter; 
    let uv_x = (iter / 24.0) + (avg_stripe * 0.5); 
    let rgb = textureSampleLevel(palette, palette_sampler, vec2(uv_x, 0.5), 0.0).rgb;
    return vec4(select(rgb.bgr, rgb, SWAP_CHANNELS), 1.0);
}

fn stripe_to_rgb(iteration: u32, x: f32, y: f32, s1: f32, s2: f32) -> vec4<f32> {
	if (iteration >= args.iterations) {
        return vec4(0.0, 0.0, 0.0, 1.0);
    }

    let nu = log2(log2(x * x + y * y) / log2(10000.0)); 
    let mx = mix(s1 / f32(iteration), s2 / f32(iteration - 1), nu);
    let iter = f32(iteration) + 1.0 - nu;
	return vec4(mx, mx, mx, 1.0);
}
