use clap::Parser;
use fract::pipeline::Pipeline;
use indicatif::{ProgressBar, ProgressStyle};
use std::{process::ExitCode, time::UNIX_EPOCH};

/// Config permutation utility.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Output collage file.
    output: Option<String>,

    /// Directory that the frames are rendered to.
    #[arg(short, long)]
    data: Option<String>,

    /// Path to a directory with the config tomls.
    #[arg(short, long)]
    configs: String,
}

fn main() -> std::io::Result<ExitCode> {
    let args = Args::parse();

    let count = std::fs::read_dir(&args.configs)?
        .filter(|p| {
            p.as_ref()
                .is_ok_and(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
        })
        .count();

    if count == 0 {
        println!("[ERROR] Directory contains no valid config files");
        return Ok(ExitCode::FAILURE);
    }

    let data_root = args.data.unwrap_or_else(|| {
        let current_time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        format!("data/{current_time}")
    });
    _ = std::fs::create_dir_all(&data_root);

    let bar = ProgressBar::new(count as u64);
    let images = 4u32.to_string().len();
    bar.set_style(
        ProgressStyle::with_template(&format!(
            "[{{elapsed_precise}}] {{bar:40.cyan/blue}} \
                frames={{pos:>{images}}}/{{len:{}}} eta={{eta_precise}}",
            images + 4,
        ))
        .unwrap()
        .progress_chars("##-"),
    );

    let first = std::fs::read_dir(&args.configs)?
        .find_map(|p| {
            p.as_ref()
                .map(|p| fract::config::from_path(p.path().to_str().unwrap()))
                .ok()
        })
        .expect("directory contains valid config tomls")?;

    let width = first.width;
    let height = first.height;

    let mut pipeline = Pipeline::new(None, first, None);
    let mut files = Vec::new();
    bar.set_position(0);
    for path in std::fs::read_dir(&args.configs)? {
        let path = path?.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            let config = fract::config::from_path(path.to_str().unwrap())?;

            if config.width != width || config.height != height {
                println!("[ERROR] All configs must have the same dimensions");
                return Ok(ExitCode::FAILURE);
            }

            pipeline.write_config(|c| {
                *c = config;
            });
            let file_stem = path.file_stem().unwrap().to_str().unwrap();
            let path = format!("{data_root}/{file_stem}.png");
            fract::render_png(&mut pipeline, None, &path, 0)?;
            bar.inc(1);
            files.push(path);
        }
    }
    bar.finish();

    println!("[LOG] Wrote {} images to {}", count, data_root);

    if let Some(output) = &args.output {
        collage(&files, width, height, output)?;
        println!(
            "[LOG] Wrote {} bytes to {}",
            std::fs::metadata(output)?.len(),
            output
        );
    }

    Ok(ExitCode::SUCCESS)
}

fn collage(files: &[String], width: usize, height: usize, output: &str) -> std::io::Result<()> {
    let count = files.len();
    let mut cols = count.isqrt();
    if cols * cols != count {
        cols += 1;
    }
    let rows = count.div_ceil(cols);
    let mut collage = vec![0u8; width * cols * height * rows * 4];

    for (i, file) in files.iter().enumerate() {
        let file = std::fs::File::open(file)?;
        let reader = std::io::BufReader::new(file);
        let mut reader = png::Decoder::new(reader).read_info()?;
        let mut frame = vec![0; reader.output_buffer_size().unwrap()];
        reader.next_frame(&mut frame).unwrap();

        let xoffset = (i % cols) * width;
        let yoffset = (i / cols) * height;

        for y in 0..height {
            let src_start = y * width * 4;
            let dest_start = ((yoffset + y) * width * cols + xoffset) * 4;
            collage[dest_start..dest_start + width * 4]
                .copy_from_slice(&frame[src_start..src_start + width * 4]);
        }
    }

    fract::encoder::png(output, &collage, width * cols, height * rows, false)
}
