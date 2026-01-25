use tint::{Color, LinearRgb, Sbgr};

// https://stackoverflow.com/a/16505538
pub fn classic() -> [Sbgr; 16] {
    [
        LinearRgb::from_rgb(66.0 / 255.0, 30.0 / 255.0, 15.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(25.0 / 255.0, 7.0 / 255.0, 26.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(9.0 / 255.0, 1.0 / 255.0, 47.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(4.0 / 255.0, 4.0 / 255.0, 73.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 7.0 / 255.0, 100.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(12.0 / 255.0, 44.0 / 255.0, 138.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(24.0 / 255.0, 82.0 / 255.0, 177.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(57.0 / 255.0, 125.0 / 255.0, 209.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(134.0 / 255.0, 181.0 / 255.0, 229.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(211.0 / 255.0, 236.0 / 255.0, 248.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(241.0 / 255.0, 233.0 / 255.0, 191.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(248.0 / 255.0, 201.0 / 255.0, 95.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 170.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(204.0 / 255.0, 128.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(153.0 / 255.0, 87.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(106.0 / 255.0, 52.0 / 255.0, 3.0 / 255.0).to_sbgr(),
    ]
}

// https://github.com/bertbaron/mandelbrot/blob/38b88b0bf5dcbe5cb214637964515197a56e124d/palette.js#L125
pub fn lava() -> [Sbgr; 24] {
    [
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(10.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(20.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(40.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(80.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(160.0 / 255.0, 10.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(200.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(240.0 / 255.0, 90.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 160.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 220.0 / 255.0, 10.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 80.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 160.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 160.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 80.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 220.0 / 255.0, 10.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 160.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(240.0 / 255.0, 90.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(200.0 / 255.0, 40.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(160.0 / 255.0, 10.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(80.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(40.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(20.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(10.0 / 255.0, 0.0 / 255.0, 0.0 / 255.0).to_sbgr(),
    ]
}

// https://github.com/bertbaron/mandelbrot/blob/38b88b0bf5dcbe5cb214637964515197a56e124d/palette.js#L148
pub fn ocean() -> [Sbgr; 18] {
    [
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 51.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 102.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 153.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 51.0 / 255.0, 102.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 102.0 / 255.0, 204.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(51.0 / 255.0, 153.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(102.0 / 255.0, 178.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(153.0 / 255.0, 204.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(204.0 / 255.0, 229.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(1.0, 1.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(204.0 / 255.0, 229.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(153.0 / 255.0, 204.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(102.0 / 255.0, 178.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(51.0 / 255.0, 153.0 / 255.0, 1.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 102.0 / 255.0, 204.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 51.0 / 255.0, 102.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 153.0 / 255.0).to_sbgr(),
        LinearRgb::from_rgb(0.0 / 255.0, 0.0 / 255.0, 102.0 / 255.0).to_sbgr(),
    ]
}
