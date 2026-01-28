use clap::Parser;
use fract::{pipeline::Pipeline, time_secs};
use indicatif::{ProgressBar, ProgressStyle};
use rug::{Float, ops::AddAssignRound};
use std::{process::ExitCode, time::UNIX_EPOCH};

/// Deep Mandelbrot set renderer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Output file, either `PNG` or `MP4`.
    output: String,

    /// Path to a config toml.
    #[arg(short, long)]
    config: Option<String>,

    /// Number of frames to render.
    #[arg(short, long, default_value_t = 1)]
    frames: usize,

    /// Number of frames per second.
    #[arg(short, long, default_value_t = 30)]
    fps: usize,

    /// Factor added to the zoom every frame.
    #[arg(short, long, default_value_t = -0.05)]
    zoom: f32,
}

fn main() -> std::io::Result<ExitCode> {
    let args = Args::parse();
    let config = args
        .config
        .and_then(|path| fract::config::from_path(&path).ok())
        .unwrap_or_default();

    if args.frames == 0 {
        println!("[ERROR] Frames must be greater than 0");
        return Ok(ExitCode::FAILURE);
    }

    let current_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let data_root = format!("data/{current_time}");
    _ = std::fs::create_dir_all(&data_root);
    let log_path = format!("{data_root}/log.txt");
    let log_file = std::fs::File::create(&log_path)?;
    let log = std::io::BufWriter::new(log_file);

    let width = config.width;
    let height = config.height;
    let mut pipeline = Pipeline::new(None, config.clone(), Some(Box::new(log)));

    let kind = if args.frames == 1 { "image" } else { "video" };
    let fps = if args.frames != 1 {
        format!("{} frames @ {}fps", args.frames, args.fps)
    } else {
        String::default()
    };
    println!("[RENDER] {}x{} {kind} {fps}", config.width, config.height,);
    config.log();

    let rt = if args.frames == 1 {
        if !args.output.to_lowercase().ends_with(".png") {
            println!("[ERROR] Invalid image format, expected PNG");
            return Ok(ExitCode::FAILURE);
        }

        let bar = ProgressBar::no_length();
        let pixels_width = pipeline.total_pixels().to_string().len();
        bar.set_style(
            ProgressStyle::with_template(&format!(
                "[{{elapsed_precise}}] {{bar:40.cyan/blue}} \
                frames={{pos:>{pixels_width}}}/{{len:{}}} eta={{eta_precise}}",
                pixels_width + 4,
            ))
            .unwrap()
            .progress_chars("##-"),
        );

        time_secs(|| fract::render_png(&mut pipeline, Some(&bar), &args.output))?
    } else {
        if !args.output.to_lowercase().ends_with(".mp4") {
            println!("[ERROR] Invalid video format, expected MP4");
            return Ok(ExitCode::FAILURE);
        }

        let bar = ProgressBar::no_length();
        let frames_width = args.frames.to_string().len();
        bar.set_style(
            ProgressStyle::with_template(&format!(
                "[{{elapsed_precise}}] {{bar:40.cyan/blue}} \
                frames={{pos:>{frames_width}}}/{{len:{}}} eta={{eta_precise}}",
                frames_width * 2,
            ))
            .unwrap()
            .progress_chars("##-"),
        );

        let encoder = fract::encoder::Encoder::new(data_root, width, height, args.fps);
        let frame_path = encoder.frame_path().to_string();
        let rt = time_secs(|| {
            fract::render_mp4(
                &mut pipeline,
                Some(&bar),
                encoder,
                args.frames,
                |z| {
                    let zoom_factor = Float::with_val(z.prec(), args.zoom);
                    let zoom_delta = Float::with_val(z.prec(), &*z * &zoom_factor);
                    z.add_assign_round(zoom_delta, rug::float::Round::Nearest);
                },
                &args.output,
            )
        })?;

        println!("[LOG] Wrote {} images to {}", args.frames, frame_path);
        rt
    };

    println!(
        "[LOG] Wrote {} bytes to {}",
        std::fs::metadata(&log_path)?.len(),
        log_path
    );
    println!(
        "[LOG] Wrote {} bytes to {}",
        std::fs::metadata(&args.output)?.len(),
        args.output
    );
    println!("[LOG] Total render time: {rt:.8}s");

    Ok(ExitCode::SUCCESS)
}
