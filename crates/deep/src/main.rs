use clap::Parser;
use compute::PRECISION;
use rug::{
    Float,
    ops::{AddAssignRound, CompleteRound},
};
use std::{process::ExitCode, time::UNIX_EPOCH};

/// Deep Mandelbrot set renderer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Number of frames to render.
    #[arg(short, long, default_value_t = 1)]
    frames: usize,

    /// Factor added to the zoom every frame.
    #[arg(short, long, default_value_t = 0.001)]
    zoom: f32,

    /// Path to a config toml.
    #[arg(short, long)]
    config: Option<String>,

    /// Output file, either `PNG` or `MP4`.
    output: String,
}

#[derive(serde::Deserialize)]
struct Config {
    x: String,
    y: String,
    zoom: String,
    iterations: usize,
    width: usize,
    height: usize,
    palette: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
            zoom: "1.0".to_string(),
            iterations: 10_000,
            width: 1600,
            height: 1600,
            palette: "classic".to_string(),
        }
    }
}

fn main() -> std::io::Result<ExitCode> {
    let args = Args::parse();

    if !args.output.ends_with(".png") && !args.output.ends_with(".mp4") {
        println!("Supported output files types are PNG and MP4");
        return Ok(ExitCode::FAILURE);
    }

    if args.frames == 0 {
        println!("Frames must be greater than 0");
        return Ok(ExitCode::FAILURE);
    }

    if args.frames == 1 && !args.output.to_lowercase().ends_with(".png") {
        println!("Invalid file type for image output, expected PNG");
        return Ok(ExitCode::FAILURE);
    }

    if args.frames > 1 && !args.output.to_lowercase().ends_with(".mp4") {
        println!("Invalid file type for video output, expected MP4");
        return Ok(ExitCode::FAILURE);
    }

    let config = if let Some(path) = args.config {
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

    let x = Float::parse(config.x).unwrap().complete(PRECISION);
    let y = Float::parse(config.y).unwrap().complete(PRECISION);
    let mut z = Float::parse(config.zoom).unwrap().complete(PRECISION);
    let iterations = config.iterations;
    let width = config.width;
    let height = config.height;
    let palette = match &*config.palette {
        "classic" => compute::palette::classic().to_vec(),
        "lava" => compute::palette::lava().to_vec(),
        "ocean" => compute::palette::ocean().to_vec(),
        _ => {
            println!("Unknown palette: {}", config.palette);
            return Ok(ExitCode::FAILURE);
        }
    };

    let mut pipeline = compute::software::Pipeline::unbuffered().super_sampled();
    let mut frame_buffer = vec![tint::Sbgr::default(); width * height];
    let fps = 30;

    if args.frames == 1 {
        time(0, || {
            compute::software::compute_mandelbrot(
                &mut pipeline,
                &mut frame_buffer,
                iterations,
                &z,
                &x,
                &y,
                &palette,
                width,
                height,
            );
            let frame_buffer = unsafe {
                std::slice::from_raw_parts(frame_buffer.as_ptr().cast(), frame_buffer.len() * 4)
            };
            png(&args.output, frame_buffer, width, height)
        })?;
        return Ok(ExitCode::SUCCESS);
    }

    let sample_rate = 48000usize;
    assert!(sample_rate.is_multiple_of(fps));
    let samples = vec![(0.0, 0.0); sample_rate / fps];

    use indicatif::ProgressBar;
    let bar = ProgressBar::new(args.frames as u64);

    let mut encoder = Encoder::new(width, height, fps, sample_rate)?;
    for _ in 0..args.frames {
        compute::software::compute_mandelbrot(
            &mut pipeline,
            &mut frame_buffer,
            iterations,
            &z,
            &x,
            &y,
            &palette,
            width,
            height,
        );
        let zoom_delta = Float::with_val(PRECISION, &z * args.zoom);
        z.add_assign_round(zoom_delta, rug::float::Round::Nearest);
        encoder.render_frame(&frame_buffer, &samples)?;
        bar.inc(1);
    }

    bar.finish();
    encoder.finish(&args.output)?;

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
    fn new(width: usize, height: usize, fps: usize, sample_rate: usize) -> std::io::Result<Self> {
        let current_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let root = format!("data/{current_time}");
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

    fn render_frame<T>(
        &mut self,
        frame_buffer: &[T],
        samples: &[(f32, f32)],
    ) -> std::io::Result<()> {
        assert_eq!(std::mem::size_of::<T>(), 4);
        assert_eq!(samples.len(), self.sample_rate / self.fps);
        assert_eq!(frame_buffer.len(), self.width * self.height);
        let output = format!("{}/frames/{}.png", self.root, self.frame_count);
        let frame_buffer = unsafe {
            std::slice::from_raw_parts(frame_buffer.as_ptr().cast(), frame_buffer.len() * 4)
        };
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
