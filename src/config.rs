#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    pub x: String,
    pub y: String,
    pub zoom: String,
    pub iterations: usize,
    pub width: usize,
    pub height: usize,
    pub palette: String,
    pub ssaa: bool,
    pub batch_iter: usize,
    pub color_scale: f32,
    pub color_mode: String,
}

impl Config {
    pub fn log(&self) {
        let ssaa = if self.ssaa { "enabled" } else { "disabled" };
        println!(
            "[CONFIG] {} iterations, method={}, palette={}, ssaa={}, \
            batch_iter={}, color_scale={}",
            self.iterations,
            self.color_mode,
            self.palette,
            ssaa,
            self.batch_iter,
            self.color_scale,
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
            zoom: "2.0".to_string(),
            iterations: 10_000,
            width: 800,
            height: 800,
            palette: "magma".to_string(),
            ssaa: false,
            batch_iter: 1000,
            color_scale: 1.0,
            color_mode: "iterations".to_string(),
        }
    }
}

pub fn from_path(path: &str) -> std::io::Result<Config> {
    match toml::from_str(&std::fs::read_to_string(path)?) {
        Ok(config) => Ok(config),
        Err(err) => {
            println!("[ERROR] Failed to parse config: {err}");
            Err(std::io::ErrorKind::Other.into())
        }
    }
}

pub fn write_to(config: &Config, path: &str) -> std::io::Result<()> {
    match toml::to_string(config) {
        Ok(config) => std::fs::write(path, config),
        Err(err) => {
            println!("[ERROR] Failed to parse config: {err}");
            Err(std::io::ErrorKind::Other.into())
        }
    }
}
