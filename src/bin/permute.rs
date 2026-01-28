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
}

#[derive(serde::Deserialize)]
struct Permute {
    iterations: Vec<usize>,
    palette: Vec<String>,
    color_scale: Vec<f32>,
}

impl Permute {
    fn len(&self) -> usize {
        self.iterations.len() * self.palette.len() * self.color_scale.len()
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
            for color_scale in permute.color_scale.iter() {
                pipeline.write_config(|config| {
                    config.iterations = *iterations;
                    config.palette = palette.clone();
                    config.color_scale = *color_scale;
                });

                fract::render_png(&mut pipeline, None, &format!("{permutes}/{i}.png"), i)?;
                fract::config::write_to(&config, &format!("{configs}/{i}.toml"))?;
                bar.inc(1);
                i += 1;
            }
        }
    }
    bar.finish();

    println!("[LOG] Wrote {} images to {}", permutations, permutes);
    println!("[LOG] Wrote {} configs to {}", permutations, configs);

    Ok(ExitCode::SUCCESS)
}
