use clap::Parser;
use fract::{config::Config, palette::parse_palette, pipeline::Pipeline, precision};
use indicatif::ProgressBar;
use rug::{
    Float,
    ops::{AddAssignRound, CompleteRound},
};
use std::{io::Write, process::ExitCode, time::UNIX_EPOCH};
use tint::Sbgr;

/// Deep Mandelbrot set renderer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Run the interactive viewer.
    #[arg(short, long, default_value_t = false)]
    viewer: bool,

    /// Number of frames to render.
    #[arg(short, long, default_value_t = 1)]
    frames: usize,

    /// Factor added to the zoom every frame.
    #[arg(short, long, default_value_t = -0.05)]
    zoom: f32,

    /// Path to a config toml.
    #[arg(short, long)]
    config: Option<String>,

    /// Output file, either `PNG` or `MP4`.
    output: Option<String>,
}

fn main() -> std::io::Result<ExitCode> {
    let mut args = Args::parse();
    let config = if let Some(path) = &args.config {
        match toml::from_str(&std::fs::read_to_string(path)?) {
            Ok(config) => config,
            Err(err) => {
                println!("Failed to parse config: {err}");
                return Ok(ExitCode::FAILURE);
            }
        }
    } else {
        Config::default()
    };

    if let Some(path) = args.output.take() {
        output(&args, &path, &config, &parse_palette(&config.palette))?;
    }

    if args.viewer {
        fract::viewer::run(fract::viewer::Memory::from_config(config));
    }

    Ok(ExitCode::SUCCESS)
}

fn output(
    args: &Args,
    output: &str,
    config: &Config,
    palette: &[Sbgr],
) -> std::io::Result<ExitCode> {
    if !output.ends_with(".png") && !output.ends_with(".mp4") {
        println!("Supported output files types are PNG and MP4");
        return Ok(ExitCode::FAILURE);
    }

    if args.frames == 0 {
        println!("Frames must be greater than 0");
        return Ok(ExitCode::FAILURE);
    }

    if args.frames == 1 && !output.to_lowercase().ends_with(".png") {
        println!("Invalid file type for image output, expected PNG");
        return Ok(ExitCode::FAILURE);
    }

    if args.frames > 1 && !output.to_lowercase().ends_with(".mp4") {
        println!("Invalid file type for video output, expected MP4");
        return Ok(ExitCode::FAILURE);
    }

    // TODO: This is stupid
    let prec = 1024 * 32;
    let z = Float::parse(&config.zoom).unwrap().complete(prec);
    let prec = precision(&z);
    let x = Float::parse(&config.x).unwrap().complete(prec);
    let y = Float::parse(&config.y).unwrap().complete(prec);
    let iterations = config.iterations;
    let width = config.width;
    let height = config.height;
    let ssaa = config.ssaa;

    let current_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let data_root = format!("data/{current_time}");
    _ = std::fs::create_dir_all(&data_root);
    let mut log = std::fs::File::create(format!("{data_root}/log.txt"))?;

    env_logger::init();
    let mut pipeline = Pipeline::new(None, palette, width, height, ssaa, x, y, z);
    let fps = 30;

    if args.frames == 1 {
        pipeline.read_position(|x, y, z| {
            log.write_all(b"[FRAME] 0\n")?;
            log.write_all(format!("x = \"{}\"\n", x.to_string_radix(10, None)).as_bytes())?;
            log.write_all(format!("y = \"{}\"\n", y.to_string_radix(10, None)).as_bytes())?;
            log.write_all(format!("zoom = \"{}\"\n\n", z.to_string_radix(10, None)).as_bytes())
        })?;

        time(0, || {
            pipeline.compute_mandelbrot(iterations);
            let pixels = pipeline.read_output_buffer_bytes();
            png(output, &pixels, width, height)
        })?;
    } else {
        let bar = ProgressBar::new(args.frames as u64);
        let zoom_factor = Float::with_val(prec, args.zoom);

        let sample_rate = 48000usize;
        assert!(sample_rate.is_multiple_of(fps));
        let samples = vec![(0.0, 0.0); sample_rate / fps];

        let mut encoder = Encoder::new(data_root, width, height, fps, sample_rate)?;
        for i in 0..args.frames {
            pipeline.read_position(|x, y, z| {
                log.write_all(format!("[FRAME] {i}\n").as_bytes())?;
                log.write_all(format!("x = \"{}\"\n", x.to_string_radix(10, None)).as_bytes())?;
                log.write_all(format!("y = \"{}\"\n", y.to_string_radix(10, None)).as_bytes())?;
                log.write_all(format!("zoom = \"{}\"\n\n", z.to_string_radix(10, None)).as_bytes())
            })?;

            pipeline.write_position(|_, _, z| {
                let zoom_delta = Float::with_val(prec, &*z * &zoom_factor);
                z.add_assign_round(zoom_delta, rug::float::Round::Nearest);
            });
            pipeline.compute_mandelbrot(iterations);
            let pixels = pipeline.read_output_buffer_bytes();
            bar.inc(1);
            encoder.render_frame(&pixels, &samples)?;
        }

        bar.finish();
        encoder.finish(output)?;
    }

    Ok(ExitCode::SUCCESS)
}

struct Encoder {
    root: String,
    audio_stream: std::io::BufWriter<std::fs::File>,
    frame_count: usize,
    width: usize,
    height: usize,
    fps: usize,
    sample_rate: usize,
}

impl Encoder {
    fn new(
        root: String,
        width: usize,
        height: usize,
        fps: usize,
        sample_rate: usize,
    ) -> std::io::Result<Self> {
        _ = std::fs::create_dir_all(format!("{root}/frames"));
        Ok(Self {
            audio_stream: std::io::BufWriter::new(std::fs::File::create(format!(
                "{root}/audio.pcm"
            ))?),
            frame_count: 0,
            width,
            height,
            fps,
            sample_rate,
            root,
        })
    }

    fn render_frame(&mut self, frame_buffer: &[u8], samples: &[(f32, f32)]) -> std::io::Result<()> {
        assert_eq!(samples.len(), self.sample_rate / self.fps);
        assert_eq!(frame_buffer.len() / 4, self.width * self.height);
        let output = format!("{}/frames/{}.png", self.root, self.frame_count);
        png(&output, frame_buffer, self.width, self.height)?;
        pcm(&mut self.audio_stream, samples)?;
        self.frame_count += 1;

        Ok(())
    }

    fn finish(self, output: &str) -> std::io::Result<()> {
        ffmpeg(&self.root, output, self.fps, self.sample_rate)
    }
}

// Fast png encoding using the rust `png` crate.
fn png(output: &str, frame: &[u8], width: usize, height: usize) -> std::io::Result<()> {
    let file = std::fs::File::create(output)?;
    let output = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(output, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    // NOTE: This is about 1% of the rendering time, which is annoying but the
    // viewer code needs the `frame_buffer` to be in `Sgbr` format...
    let frame: Vec<u8> = frame
        .chunks_exact(4)
        .flat_map(|bgr| [bgr[2], bgr[1], bgr[0], bgr[3]])
        .collect();
    writer.write_image_data(&frame).unwrap();

    Ok(())
}

// PCM is what I am calling it, however, it is just all of the samples copied
// into a file: https://en.wikipedia.org/wiki/Pulse-code_modulation
fn pcm(output: &mut impl std::io::Write, samples: &[(f32, f32)]) -> std::io::Result<()> {
    for (l, r) in samples.iter() {
        output.write_all(&l.to_le_bytes())?;
        output.write_all(&r.to_le_bytes())?;
    }
    output.flush()?;
    Ok(())
}

fn ffmpeg(root: &str, output: &str, fps: usize, sample_rate: usize) -> std::io::Result<()> {
    let fps = &format!("{fps}");
    let sample_rate = &format!("{sample_rate}");
    let frames = &format!("{root}/frames/%d.png");
    let audio = &format!("{root}/audio.pcm");
    #[rustfmt::skip]
    std::process::Command::new("ffmpeg")
        .args([
            "-framerate", fps,
            "-i", frames,
            "-f", "f32le",
            "-ar", sample_rate,
            "-ac", "2",
            "-i", audio,
            "-c:v", "libx264",
            "-preset", "medium",
            "-crf", "23",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac",
            "-b:a", "192k",
            "-shortest",
            output,
        ])
            .spawn()
            .unwrap()
            .wait()?;
    Ok(())
}

fn time<R>(frame: usize, f: impl FnOnce() -> R) -> R {
    let start = std::time::SystemTime::now();
    let r = f();
    let s = std::time::SystemTime::now()
        .duration_since(start)
        .unwrap()
        .as_secs_f32();
    println!("Rendered frame {frame} in {s:.8}s");
    r
}
