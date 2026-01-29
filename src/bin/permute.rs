use clap::Parser;
use std::process::ExitCode;

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

    /// Seperate output by palette.
    #[arg(short, long, default_value_t = false)]
    split: bool,
}

#[derive(serde::Deserialize)]
struct Permute {
    iterations: Vec<usize>,
    palette: Vec<String>,
    color_scale: ColorScalePermute,
}

#[derive(serde::Deserialize)]
struct ColorScalePermute {
    start: f32,
    end: f32,
    steps: usize,
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
    let mut config = args
        .config
        .and_then(|path| fract::config::from_path(&path).ok())
        .unwrap_or_default();
    let permute = from_path(&args.permute)?;
    if permute.color_scale.steps == 0 {
        println!("[ERROR] color_scale steps must be 1 or more");
        return Ok(ExitCode::FAILURE);
    }

    let configs = format!("{}/configs", args.output);
    _ = std::fs::create_dir_all(&configs);

    let mut i = 0;
    let mut total = 0;
    for palette in permute.palette.iter() {
        if args.split {
            _ = std::fs::create_dir_all(format!("{configs}/{palette}"));
            i = 0;
        }

        let split = if args.split {
            format!("/{}", palette)
        } else {
            "".to_string()
        };

        for iterations in permute.iterations.iter() {
            let inc = if permute.color_scale.steps == 1 {
                0.0
            } else {
                (permute.color_scale.end - permute.color_scale.start)
                    / (permute.color_scale.steps as f32 - 1.0)
            };

            for step in 0..permute.color_scale.steps {
                config.iterations = *iterations;
                config.palette = palette.clone();
                config.color_scale = permute.color_scale.start + inc * step as f32;

                fract::config::write_to(&config, &format!("{configs}{split}/{i}.toml"))?;
                i += 1;
                total += 1;
            }
        }
    }

    println!("[LOG] Wrote {} configs to {}", total, configs);

    Ok(ExitCode::SUCCESS)
}
