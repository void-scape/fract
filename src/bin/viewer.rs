use clap::Parser;
use std::process::ExitCode;

/// Mandelbrot set viewer.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to a config toml.
    #[arg(short, long)]
    config: Option<String>,
}

fn main() -> std::io::Result<ExitCode> {
    let args = Args::parse();
    let config = args
        .config
        .and_then(|path| fract::config::from_path(&path).ok())
        .unwrap_or_default();
    config.log();

    let memory = fract::viewer::Memory::from_config(config);
    fract::viewer::run(memory);
}
