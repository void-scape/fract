// https://stackoverflow.com/a/16505538
const classic = array<vec3<f32>, 16>(
	vec3(66, 30, 15) / 255.0,
	vec3(25, 7, 26) / 255.0,
	vec3(9, 1, 47) / 255.0,
	vec3(4, 4, 73) / 255.0,
	vec3(0, 7, 100) / 255.0,
	vec3(12, 44, 138) / 255.0,
	vec3(24, 82, 177) / 255.0,
	vec3(57, 125, 209) / 255.0,
	vec3(134, 181, 229) / 255.0,
	vec3(211, 236, 248) / 255.0,
	vec3(241, 233, 191) / 255.0,
	vec3(248, 201, 95) / 255.0,
	vec3(255, 170, 0) / 255.0,
	vec3(204, 128, 0) / 255.0,
	vec3(153, 87, 0) / 255.0,
	vec3(106, 52, 3) / 255.0,
);

// https://github.com/bertbaron/mandelbrot/blob/38b88b0bf5dcbe5cb214637964515197a56e124d/palette.js#L125
const lava = array<vec3<f32>, 24>(
	vec3(0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(10.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(20.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(40.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(80.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(160.0 / 255.0, 10.0 / 255.0, 0.0 / 255.0),
	vec3(200.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0),
	vec3(240.0 / 255.0, 90.0 / 255.0, 0.0 / 255.0),
	vec3(1.0, 160.0 / 255.0, 0.0 / 255.0),
	vec3(1.0, 220.0 / 255.0, 10.0 / 255.0),
	vec3(1.0, 1.0, 80.0 / 255.0),
	vec3(1.0, 1.0, 160.0 / 255.0),
	vec3(1.0, 1.0, 1.0),
	vec3(1.0, 1.0, 160.0 / 255.0),
	vec3(1.0, 1.0, 80.0 / 255.0),
	vec3(1.0, 220.0 / 255.0, 10.0 / 255.0),
	vec3(1.0, 160.0 / 255.0, 0.0 / 255.0),
	vec3(240.0 / 255.0, 90.0 / 255.0, 0.0 / 255.0),
	vec3(200.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0),
	vec3(160.0 / 255.0, 10.0 / 255.0, 0.0 / 255.0),
	vec3(80.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(40.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(20.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
	vec3(10.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0),
);

fn iteration_to_rgb(iteration: u32, x: f32, y: f32) -> vec4<f32> {
	let palette = lava;
	let len: u32 = 24;

    if (iteration == args.max_iteration) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

	// https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring
    let zn = dot(vec2(x, y), vec2(x, y));
    let nu = log2(log2(zn) * 0.5);
    let iter = f32(iteration) + 1.0 - nu;

    let index = iter % f32(len);
    let c1 = u32(floor(index));
    let c2 = (c1 + 1) % len;
    let t = fract(index);
    let rgb = mix(palette[c1], palette[c2], t);
    return vec4<f32>(select(rgb.bgr, rgb, SWAP_CHANNELS), 1.0);
}
