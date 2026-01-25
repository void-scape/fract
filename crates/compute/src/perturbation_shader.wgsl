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
	initial_q: i32,
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
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
	out.delta = vec2(out.clip_position.x, -out.clip_position.y);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 1. Separate the 'integer' q for bit-shifting and the 'float' zoom for scaling
    // Let's assume args.initial_q is now a float (e.g., 15.5)
    let zoom_exponent = args.initial_q;
    let zoom_fraction = args.initial_q - zoom_exponent;

    // The starting power-of-2 scale
    var q = i32(zoom_exponent) - 1; 
    let cq = f32(q); 
    
    // 2. Apply the fractional part to the deltas immediately
    // This removes the "stepped" feel of the power-of-2 zoom
    let smooth_scale = pow(2.0, -zoom_fraction);
    var dcx = in.delta.x * smooth_scale;
    var dcy = in.delta.y * smooth_scale;

    var dx = 0.0;
    var dy = 0.0;
    var j = 0;
    var k = 0;
    
    // Initial orbit reference
    var x = orbit[k].dx;
    var y = orbit[k].dy;
    var S = pow(2.0, f32(q));

    for (var i = 0; i < i32(args.max_iteration); i++) {
        j += 1;
        k += 1;
        
        let os = orbit[k - 1].exponent;
        
        // Re-scale deltas based on the reference orbit's exponent change
        let scale_factor = pow(2.0, f32(-os));
        dcx *= scale_factor;
        dcy *= scale_factor;

        // unS handles the relative scale between the delta and the reference orbit
        var unS = pow(2.0, f32(q) - f32(os));
        if (abs(unS) > 3.4e34) { unS = 0.0; }

        // Perturbation Iteration: z + dz -> (z + dz)^2 + c + dc
        // (x + dx)^2 = x^2 + 2x*dx + dx^2
        let tx = 2.0 * x * dx - 2.0 * y * dy + unS * (dx * dx - dy * dy) + dcx;
        dy = 2.0 * x * dy + 2.0 * y * dx + unS * 2.0 * dx * dy + dcy;
        dx = tx;

        q += i32(os);
        S = pow(2.0, f32(q));
        x = orbit[k].dx;
        y = orbit[k].dy;

        // Combined position for bailout check
        let fx = x * pow(2.0, f32(orbit[k].exponent)) + S * dx;
        let fy = y * pow(2.0, f32(orbit[k].exponent)) + S * dy;

        if (fx * fx + fy * fy > 4.0) { break; }

        // 3. Renormalization (keeping dx/dy within float range)
        if (dx * dx + dy * dy > 1.0e6) {
            dx *= 0.5;
            dy *= 0.5;
            q += 1;
            dcx *= 0.5;
            dcy *= 0.5;
            S = pow(2.0, f32(q));
        }

        // 4. Rebasing: If delta is too large, reset to a new reference point
        if ((fx * fx + fy * fy < S * S * (dx * dx + dy * dy)) || (k >= (i32(args.orbit_len) - 1))) {
            dx = fx;
            dy = fy;
            q = 0; 
            S = 1.0;
            // Recalculate dcx/dcy relative to the new base scale
            dcx = (in.delta.x * smooth_scale) * pow(2.0, -f32(q) + cq);
            dcy = (in.delta.y * smooth_scale) * pow(2.0, -f32(q) + cq);
            k = 0;
            x = orbit[0].dx;
            y = orbit[0].dy;
        }
    }

 	let c = (f32(args.max_iteration) - f32(j)) / 20.0;
 	return vec4(vec3(cos(c), cos(1.1214 * c), cos(0.8 * c)) / -2.0 + 0.5, 1.0);
}

// // I am not going to pretend to understand this code: 
// // https://github.com/HastingsGreer/mandeljs/blob/7bb12c6ee2214e4eea82a30498de85823b3be474/main.js#L198
// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
// 	var q = args.initial_q - 1;
// 	let cq = q;
// 	var S = pow(2.0, f32(q));
// 	var dcx = in.delta.x;
// 	var dcy = in.delta.y;
// 
// 	var dx = 0.0;
// 	var dy = 0.0;
// 
// 	var j = 0;
// 	var k = 0;
// 
// 	var x = orbit[k].dx;
// 	var y = orbit[k].dy;
// 
// 	for (var i = k; i < i32(args.max_iteration); i++) {
// 		j += 1;
// 		k += 1;
// 
// 		let os = orbit[k - 1].exponent;
// 		dcx = in.delta.x * pow(2.0, f32(-q + cq - os));
// 		dcy = in.delta.y * pow(2.0, f32(-q + cq - os));
// 		var unS = pow(2.0, f32(q) - f32(os));
// 
// 		if (abs(unS) > 3.4028235e34) {
// 			unS = 0.0;
// 		}
// 
// 		let tx = 2.0 * x * dx - 2.0 * y * dy + unS * dx * dx - unS * dy * dy + dcx;
// 		dy = 2.0 * x * dy + 2.0 * y * dx + unS * 2.0 * dx * dy + dcy;
// 		dx = tx;
// 
// 		q = q + os;
// 		S = pow(2.0, f32(q));
// 
// 		x = orbit[k].dx;
// 		y = orbit[k].dy;
// 		let fx = x * pow(2.0, f32(orbit[k].exponent)) + S * dx;
// 		let fy = y * pow(2.0, f32(orbit[k].exponent)) + S * dy;
// 		if (fx * fx + fy * fy > 10000.0) {
// 			break;
// 		}
// 
// 		if (dx * dx + dy * dy > 1000000.0) {
// 			dx = dx / 2.0;
// 			dy = dy / 2.0;
// 			q = q + 1;
// 			S = pow(2.0, f32(q));
// 			dcx = in.delta.x * pow(2.0, f32(-q + cq));
// 			dcy = in.delta.y * pow(2.0, f32(-q + cq));
// 		}
// 
// 		if (
// 			(fx * fx + fy * fy < S * S * dx * dx + S * S * dy * dy) 
// 				|| (k >= (i32(args.orbit_len) - 1))
// 		) {
// 			dx = fx;
// 			dy = fy;
// 			q = 0;
// 			S = pow(2.0, f32(q));
// 			dcx = in.delta.x * pow(2.0, f32(-q + cq));
// 			dcy = in.delta.y * pow(2.0, f32(-q + cq));
// 			k = 0;
// 			x = orbit[0].dx;
// 			y = orbit[0].dy;
// 		}
// 	}
// 
// 	let c = (f32(args.max_iteration) - f32(j)) / 20.0;
// 	return vec4(vec3(cos(c), cos(1.1214 * c), cos(0.8 * c)) / -2.0 + 0.5, 1.0);
// }

// @fragment
// fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
//     let px = in.uv.x * f32(args.width);
//     let py = (1.0 - in.uv.y) * f32(args.height);
// 
// 	let dx0 = args.sdx + px * args.xstep;
// 	let dy0 = args.sdy + py * args.ystep;
// 
// 	// Compute the delta of (x0, y0) with respect to the
// 	// reference orbit.
// 	var dx = dx0;
// 	var dy = dy0;
// 	var iteration = 0u;
// 	var ref_iteration = 0u;
// 
// 	// If there are coefficients present, approximate the position
// 	// of (dx, dy) at iteration `approx_iteration`.
// 	if args.approx_iteration > 0 {
// 		// D = Ad + Bd^2 + Cd^3
// 		let d = dx0;
// 		let di = dy0;
// 
// 		let d2 = d * d - di * di;
// 		let d2i = di * d + d * di;
// 
// 		let d3 = d2 * d - d2i * di;
// 		let d3i = d2 * di + d2i * d;
// 
// 		let ad = args.ax * d - args.ay * di;
// 		let adi = args.ay * d + args.ax * di;
// 
// 		let bd2 = args.bx * d2 - args.by * d2i;
// 		let bd2i = args.by * d2 + args.bx * d2i;
// 
// 		let cd3 = args.cxx * d3 - args.cyy * d3i;
// 		let cd3i = args.cyy * d3 + args.cxx * d3i;
// 
// 		dx = ad + bd2 + cd3;
// 		dy = adi + bd2i + cd3i;
// 
// 		iteration = args.approx_iteration;
// 		ref_iteration = args.approx_iteration;
// 	}
// 
// 	while iteration < args.max_iteration {
// 		var ax = orbit[ref_iteration * 2];
// 		var ay = orbit[ref_iteration * 2 + 1];
// 		ax *= 2.0;
// 		ay *= 2.0;
// 
// 		// ad = a * d
// 		let adx = ax * dx - ay * dy;
// 		let ady = ax * dy + ay * dx;
// 
// 		// a = a * d + d * d
// 		ax = adx + dx * dx - dy * dy;
// 		ay = ady + dx * dy + dy * dx;
// 
// 		// d = a * d + d * d + d0
// 		dx = ax + dx0;
// 		dy = ay + dy0;
// 
// 		ref_iteration += 1;
// 
// 		// The full value of (x0, y0) is reconstructed from
// 		// the reference orbit and checked for escape time.
// 		let x = orbit[ref_iteration * 2];
// 		let y = orbit[ref_iteration * 2 + 1];
// 		let zmag = (dx + x) * (dx + x) + (dy + y) * (dy + y);
// 		let dmag = dx * dx + dy * dy;
// 
// 		if zmag > 10000.0 {
// 			break;
// 		} else if zmag < dmag || ref_iteration == args.orbit_len - 1 {
// 			dx += x;
// 			dy += y;
// 			ref_iteration = 0;
// 		}
// 
// 		iteration += 1;
// 	}
//     
// 	let x = orbit[ref_iteration * 2];
// 	let y = orbit[ref_iteration * 2 + 1];
// 	return iteration_to_rgb(iteration, x + dx, y + dy);
// }

