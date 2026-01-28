use clap::Parser;
use fract::pipeline::Pipeline;
use indicatif::{ProgressBar, ProgressStyle};
use std::{process::ExitCode, time::UNIX_EPOCH};

/// Config permutation utility.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Output directory.
    output: String,

    /// Path to a config toml to initialize the permutations.
    #[arg(short, long)]
    config: Option<String>,

    /// Path to a permutation config toml.
    #[arg(short, long)]
    permute: String,

    /// Open the collage after rendering.
    #[arg(short, long, default_value_t = false)]
    open: bool,

    /// Seperate collages by palette.
    #[arg(short, long, default_value_t = false)]
    split: bool,
}

#[derive(serde::Deserialize)]
struct Permute {
    iterations: Vec<usize>,
    palette: Vec<String>,
    color_scale: ColorScalePermute,
}

impl Permute {
    fn len(&self) -> usize {
        self.iterations.len() * self.palette.len() * self.color_scale.len()
    }
}

#[derive(serde::Deserialize)]
struct ColorScalePermute {
    start: f32,
    end: f32,
    steps: usize,
}

impl ColorScalePermute {
    fn len(&self) -> usize {
        self.steps
    }
}

fn from_path(path: &str) -> std::io::Result<Permute> {
    match toml::from_str(&std::fs::read_to_string(path)?) {
        Ok(config) => Ok(config),
        Err(err) => {
            println!("[ERROR] Failed to parse config: {err}");
            Err(std::io::ErrorKind::Other.into())
        }
    }
}

fn main() -> std::io::Result<ExitCode> {
    let args = Args::parse();
    let config = args
        .config
        .and_then(|path| fract::config::from_path(&path).ok())
        .unwrap_or_default();
    let permute = from_path(&args.permute)?;
    if permute.color_scale.steps == 0 {
        println!("[ERROR] color_scale steps must be 1 or more");
        return Ok(ExitCode::FAILURE);
    }

    let current_time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let data_root = format!("{}/{current_time}", args.output);
    let permutes = format!("{}/frames", data_root);
    let configs = format!("{}/configs", data_root);
    _ = std::fs::create_dir_all(&permutes);
    _ = std::fs::create_dir_all(&configs);

    let permutations = permute.len();
    let bar = ProgressBar::new(permutations as u64);
    let images = 4u32.to_string().len();
    bar.set_style(
        ProgressStyle::with_template(&format!(
            "[{{elapsed_precise}}] {{bar:40.cyan/blue}} \
                frames={{pos:>{images}}}/{{len:{}}}",
            images + 4,
        ))
        .unwrap()
        .progress_chars("##-"),
    );

    let mut pipeline = Pipeline::new(None, config.clone(), None);

    bar.set_position(0);
    let mut i = 0;
    for iterations in permute.iterations.iter() {
        for palette in permute.palette.iter() {
            if args.split {
                _ = std::fs::create_dir_all(format!("{permutes}/{palette}"));
                _ = std::fs::create_dir_all(format!("{configs}/{palette}"));
                i = 0;
            }

            let inc = if permute.color_scale.steps == 1 {
                0.0
            } else {
                (permute.color_scale.end - permute.color_scale.start)
                    / (permute.color_scale.steps as f32 - 1.0)
            };

            for step in 0..permute.color_scale.steps {
                pipeline.write_config(|config| {
                    config.iterations = *iterations;
                    config.palette = palette.clone();
                    config.color_scale = permute.color_scale.start + inc * step as f32;
                });

                let split = if args.split {
                    format!("/{}", palette)
                } else {
                    "".to_string()
                };
                fract::render_png(
                    &mut pipeline,
                    None,
                    &format!("{permutes}{split}/{i}.png"),
                    i,
                )?;
                pipeline.read_config(|config| {
                    fract::config::write_to(config, &format!("{configs}{split}/{i}.toml"))
                })?;
                bar.inc(1);
                i += 1;
            }
        }
    }
    bar.finish();

    collage(
        &data_root,
        config.width,
        config.height,
        permutations,
        &permute.palette,
        args.split,
    )?;

    println!("[LOG] Wrote {} images to {}", permutations, permutes);
    println!("[LOG] Wrote {} configs to {}", permutations, configs);

    if args.open && !args.split {
        // TODO: os specific
        let output = format!("{data_root}/collage.png");
        std::process::Command::new("open")
            .arg(output)
            .spawn()
            .expect("failed to open output")
            .wait()
            .map(|_| ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::SUCCESS)
    }
}

fn collage(
    data_root: &str,
    width: usize,
    height: usize,
    count: usize,
    palettes: &[String],
    split: bool,
) -> std::io::Result<()> {
    if split {
        let count = count / palettes.len();

        let mut cols = count.isqrt();
        if cols * cols != count {
            cols += 1;
        }
        let rows = count.div_ceil(cols);
        let mut collage = vec![0u8; width * cols * height * rows * 4];

        for palette in palettes.iter() {
            for i in 0..count {
                let file = std::fs::File::open(format!("{data_root}/frames/{palette}/{i}.png"))?;
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

            let output = format!("{data_root}/{palette}_collage.png");
            fract::encoder::png(&output, &collage, width * cols, height * rows, false)?;
            println!(
                "[LOG] Wrote {} bytes to {}",
                std::fs::metadata(&output)?.len(),
                output
            );
        }
    } else {
        let mut cols = count.isqrt();
        if cols * cols != count {
            cols += 1;
        }
        let rows = count.div_ceil(cols);
        let mut collage = vec![0u8; width * cols * height * rows * 4];

        for i in 0..count {
            let file = std::fs::File::open(format!("{data_root}/frames/{i}.png"))?;
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

        let output = format!("{data_root}/collage.png");
        fract::encoder::png(&output, &collage, width * cols, height * rows, false)?;
        println!(
            "[LOG] Wrote {} bytes to {}",
            std::fs::metadata(&output)?.len(),
            output
        );
    }

    Ok(())
}
