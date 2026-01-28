/// Encodes a series of png images into an mp4 video.
pub struct Encoder {
    data_dir: String,
    width: usize,
    height: usize,
    fps: usize,
    frame: usize,
}

impl Encoder {
    pub fn new(data_dir: String, width: usize, height: usize, fps: usize) -> Self {
        _ = std::fs::create_dir_all(format!("{data_dir}/frames"));
        Self {
            data_dir,
            width,
            height,
            fps,
            frame: 0,
        }
    }

    pub fn frame_path(&self) -> &str {
        &self.data_dir
    }

    /// Save `frame_buffer` to a png in `data_dir`.
    pub fn render_frame(&mut self, frame_buffer: &[u8]) -> std::io::Result<()> {
        let output = format!("{}/frames/{}.png", self.data_dir, self.frame);
        png(&output, frame_buffer, self.width, self.height, true)?;
        self.frame += 1;
        Ok(())
    }

    /// Generate an mp4 video from the rendered frames in `data_dir`.
    pub fn finish(self, output: &str) -> std::io::Result<()> {
        ffmpeg(&self.data_dir, output, self.fps)
    }
}

// Fast png encoding using the rust `png` crate.
pub fn png(
    output: &str,
    frame: &[u8],
    width: usize,
    height: usize,
    flip_channels: bool,
) -> std::io::Result<()> {
    let file = std::fs::File::create(output)?;
    let output = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(output, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    if flip_channels {
        let frame: Vec<u8> = frame
            .chunks_exact(4)
            .flat_map(|bgr| [bgr[2], bgr[1], bgr[0], bgr[3]])
            .collect();
        writer.write_image_data(&frame).unwrap();
    } else {
        writer.write_image_data(frame).unwrap();
    }

    Ok(())
}

fn ffmpeg(root: &str, output: &str, fps: usize) -> std::io::Result<()> {
    let fps = &format!("{fps}");
    let frames = &format!("{root}/frames/%d.png");
    #[rustfmt::skip]
    std::process::Command::new("ffmpeg")
        .args([
            "-framerate", fps,
            "-i", frames,
            "-c:v", "libx264",
            "-preset", "medium",
            "-crf", "23",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac",
            "-b:a", "192k",
            output,
        ])
            .spawn()
            .unwrap()
            .wait()?;
    Ok(())
}
