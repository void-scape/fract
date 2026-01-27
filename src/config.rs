#[derive(serde::Deserialize)]
pub struct Config {
    pub x: String,
    pub y: String,
    pub zoom: String,
    pub iterations: usize,
    pub width: usize,
    pub height: usize,
    pub palette: String,
    pub ssaa: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            x: "0.0".to_string(),
            y: "0.0".to_string(),
            zoom: "2.0".to_string(),
            iterations: 10_000,
            width: 1600,
            height: 1600,
            palette: "classic".to_string(),
            ssaa: false,
        }
    }
}
