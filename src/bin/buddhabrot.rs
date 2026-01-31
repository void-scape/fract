use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::atomic::{AtomicU32, Ordering};
use tint::Srgb;

/// Buddhabrot renderer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Output png file.
    output: String,

    /// Path to a config toml.
    #[arg(short, long)]
    config: String,

    /// Path to the data directory.
    #[arg(short, long)]
    data: String,
}

const DEFAULT_SIZE: usize = 800;
const DEFAULT_ITERATIONS: usize = 1_000;
const DEFAULT_COLOR_SCALE: f32 = 8_000.0;
const DEFAULT_CHANNEL_SCALE: f32 = 1.0;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "kebab-case")]
struct Config {
    size: usize,
    r_channel: Channel,
    g_channel: Channel,
    b_channel: Channel,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            size: DEFAULT_SIZE,
            r_channel: Channel::default(),
            g_channel: Channel::default(),
            b_channel: Channel::default(),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(default)]
struct Channel {
    scale: f32,
    iterations: usize,
    color_scale: f32,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            scale: DEFAULT_CHANNEL_SCALE,
            iterations: DEFAULT_ITERATIONS,
            color_scale: DEFAULT_COLOR_SCALE,
        }
    }
}

fn find_channel_hist(
    size: usize,
    channel: &Channel,
    data: &str,
) -> std::io::Result<Option<Vec<u32>>> {
    let mut out = None;
    let target_stem = format!("{}-{}", size, channel.iterations);
    for entry in std::fs::read_dir(data)? {
        if entry?
            .path()
            .file_stem()
            .is_some_and(|str| *str == *target_stem)
        {
            let bin = std::fs::read(format!("{}/{}.bin", data, target_stem))?;
            out = Some(
                bin.chunks(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect(),
            );
        }
    }
    Ok(out)
}

fn process_channel(
    size: usize,
    channel_config: &Channel,
    data_dir: &str,
    multi: &MultiProgress,
) -> std::io::Result<Vec<u32>> {
    if let Some(existing) = find_channel_hist(size, channel_config, data_dir)? {
        return Ok(existing);
    }

    let iterations = channel_config.iterations;
    let bar = multi.add(progress_bar(size, iterations));
    let hist = compute_hist(Some(&bar), size, iterations);

    let target_stem = format!("{}-{}", size, iterations);
    std::fs::write(
        format!("{}/{}.bin", data_dir, target_stem),
        fract::byte_slice(&hist),
    )?;

    Ok(hist)
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    let config = toml::from_str::<Config>(&std::fs::read_to_string(args.config)?).unwrap();

    let multi = MultiProgress::new();
    let (r, (g, b)) = rayon::join(
        || process_channel(config.size, &config.r_channel, &args.data, &multi),
        || {
            rayon::join(
                || process_channel(config.size, &config.g_channel, &args.data, &multi),
                || process_channel(config.size, &config.b_channel, &args.data, &multi),
            )
        },
    );
    let r = r?;
    let g = g?;
    let b = b?;

    let mut frame_buffer = vec![Srgb::default(); config.size * config.size];
    let color_scale = [
        config.r_channel.color_scale,
        config.g_channel.color_scale,
        config.b_channel.color_scale,
    ];
    for (i, pixel) in frame_buffer.iter_mut().enumerate() {
        let rgb = [
            (r[i] as f32 / color_scale[0]).powf(1.2),
            (g[i] as f32 / color_scale[1]).powf(1.2),
            (b[i] as f32 / color_scale[2]).powf(1.2),
        ];
        *pixel = Srgb::from_rgb(
            ((rgb[0] * config.r_channel.scale).clamp(0.0, 1.0) * 255.0) as u8,
            ((rgb[1] * config.g_channel.scale).clamp(0.0, 1.0) * 255.0) as u8,
            ((rgb[2] * config.b_channel.scale).clamp(0.0, 1.0) * 255.0) as u8,
        );
    }

    fract::encoder::png(
        &args.output,
        fract::byte_slice(&frame_buffer),
        config.size,
        config.size,
        false,
    )?;

    Ok(())
}

fn progress_bar(size: usize, iter: usize) -> ProgressBar {
    let bar = ProgressBar::no_length();
    let width = (size * OVERY as usize / 2).to_string().len();
    bar.set_style(
        ProgressStyle::with_template(&format!(
            "[{{elapsed_precise}}] iter={iter:<8} {{bar:40.cyan/blue}} \
                cols={{pos:>{width}}}/{{len:{}}} eta={{eta_precise}}",
            width * 2,
        ))
        .unwrap()
        .progress_chars("##-"),
    );
    bar
}

const SPANX: f32 = 3.5;
const SPANY: f32 = 3.5;
const XOFFSET: f32 = -0.25;

const OVERX: f32 = 32.0;
const OVERY: f32 = 32.0;

fn compute_hist(progress_bar: Option<&ProgressBar>, size: usize, iterations: usize) -> Vec<u32> {
    if let Some(bar) = progress_bar {
        bar.set_length((size * OVERY as usize / 2) as u64);
        bar.set_position(0);
    }

    let hist = (0..size * size)
        .map(|_| AtomicU32::new(0))
        .collect::<Vec<_>>();

    let fsize = size as f32;
    let total_pixels = (1 + size * OVERY as usize / 2) * (size * OVERX as usize);
    (0..total_pixels).into_par_iter().for_each(|i| {
        let py = i / (size * OVERX as usize);
        let px = i % (size * OVERX as usize);

        // only computes one half of the y-axis then copies to the other
        let y0 = (py as f32 - fsize * OVERY / 2.0) / (fsize * OVERY) * SPANY;
        let x0 = (px as f32 - fsize * OVERX / 2.0) / (fsize * OVERX) * SPANX + XOFFSET;

        // simple cardioid and bulb check
        //
        // https://mathr.co.uk/blog/2022-11-19_cardioid_and_bulb_checking.html
        let y2 = y0 * y0;
        let q = (x0 - 0.25).powi(2) + y2;
        if q * (q + (x0 - 0.25)) < 0.25 * y2 || (x0 + 1.0).powi(2) + y2 < 0.25 * 0.25 {
            return;
        }

        let mut path = Vec::new();
        let mut x = 0.0;
        let mut y = 0.0;
        let mut iteration = 0;
        while iteration < iterations {
            let x2 = x * x;
            let y2 = y * y;
            if x2 + y2 > 10000.0 {
                break;
            }
            y = 2.0 * x * y + y0;
            x = x2 - y2 + x0;
            iteration += 1;
            path.push((x, y));
        }

        if iteration == iterations {
            return;
        }

        for (x, y) in path.iter() {
            let px = (((x - XOFFSET) / SPANX + 0.5) * fsize) as isize;
            if px < 0 || px >= size as isize {
                continue;
            }

            let py = ((y / SPANY + 0.5) * fsize) as isize;
            if py < 0 || py >= size as isize {
                continue;
            }

            // rotate, this works because width == height
            let x = px as usize;
            let y = py as usize;
            // write to both sides of the y-axis
            hist[x * size + (size - 1 - y)].fetch_add(1, Ordering::Relaxed);
            hist[x * size + y].fetch_add(1, Ordering::Relaxed);
        }

        if i % (size * OVERX as usize) == 0
            && let Some(bar) = progress_bar
        {
            bar.inc(1);
        }
    });
    if let Some(bar) = progress_bar {
        bar.finish();
    }
    // SAFETY: Nobody has access to hist at this point in time.
    unsafe { std::mem::transmute(hist) }
}
