use compute::PRECISION;
use rug::{
    Float,
    ops::{AddAssignRound, CompleteRound},
};
use std::io::Write;

fn main() -> std::io::Result<()> {
    let mut pipeline = compute::software::Pipeline::default();

    let width = compute::WIDTH;
    let height = compute::HEIGHT;
    let fps = 30usize;
    let mut frame_buffer = vec![tint::Srgb::default(); width * height];

    let x = Float::parse(
        "-1.769081456227405031584376710555741599567825506942627814698025962013971064256665544200937483400440897",
    ).unwrap().complete(PRECISION);
    let y = Float::parse(
        "0.003037787152539119673394881819412740681023744946458660316743819462018920571341488787781769620915429870",
    ).unwrap().complete(PRECISION);
    let mut zoom = Float::parse(
        "0.000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000061363668308",
    ).unwrap().complete(PRECISION);
    let iterations = 30000;

    // let x = Float::with_val(compute::PRECISION, 0.0);
    // let y = Float::with_val(compute::PRECISION, 0.0);
    // let mut zoom = Float::with_val(compute::PRECISION, 1.0);

    let sample_rate = 48000usize;
    assert!(sample_rate.is_multiple_of(fps));
    let samples = vec![(0.0, 0.0); sample_rate / fps];

    let mut encoder = Encoder::new(width, height, fps, sample_rate)?;
    for i in 0..fps * 60 {
        let start = std::time::SystemTime::now();

        compute::software::compute_mandelbrot(
            &mut pipeline,
            &mut frame_buffer,
            iterations,
            &zoom,
            &x,
            &y,
        );
        let zoom_delta = Float::with_val(PRECISION, &zoom * 0.0005);
        zoom.add_assign_round(zoom_delta, rug::float::Round::Nearest);
        encoder.render_frame(&frame_buffer, &samples)?;

        let s = std::time::SystemTime::now()
            .duration_since(start)
            .unwrap()
            .as_secs_f32();
        println!("rendered frame {i} in {s:.2}s");
    }

    encoder.finish()
}

struct Encoder {
    audio_stream: std::io::BufWriter<std::fs::File>,
    frame_count: usize,
    width: usize,
    height: usize,
    fps: usize,
    sample_rate: usize,
}

impl Encoder {
    fn new(width: usize, height: usize, fps: usize, sample_rate: usize) -> std::io::Result<Self> {
        _ = std::fs::remove_dir_all("data");
        std::fs::create_dir_all("data/frames")?;

        Ok(Self {
            audio_stream: std::io::BufWriter::new(std::fs::File::create("data/audio.pcm")?),
            frame_count: 0,
            width,
            height,
            fps,
            sample_rate,
        })
    }

    fn render_frame<T>(
        &mut self,
        frame_buffer: &[T],
        samples: &[(f32, f32)],
    ) -> std::io::Result<()> {
        assert_eq!(std::mem::size_of::<T>(), 4);
        assert_eq!(samples.len(), self.sample_rate / self.fps);
        assert_eq!(frame_buffer.len(), self.width * self.height);
        let output = format!("data/frames/{}.ppm", self.frame_count);
        let frame_buffer = unsafe { std::mem::transmute::<&[T], &[u32]>(frame_buffer) };
        ppm(&output, frame_buffer, self.width, self.height)?;
        pcm(&mut self.audio_stream, samples)?;
        self.frame_count += 1;

        Ok(())
    }

    fn finish(self) -> std::io::Result<()> {
        ffmpeg(self.fps, self.sample_rate)
    }
}

// Stupid simple self describing image format. As such, these files are pretty
// large: https://en.wikipedia.org/wiki/Netpbm
fn ppm(output: &str, frame: &[u32], width: usize, height: usize) -> std::io::Result<()> {
    let file = std::fs::File::create(output)?;
    let mut output = std::io::BufWriter::new(file);
    output.write_all(format!("P6\n{width} {height}\n255\n").as_bytes())?;
    for row in frame.chunks(width) {
        for pixel in row {
            output.write_all(&[
                ((pixel >> 24) & 0xFF) as u8,
                ((pixel >> 16) & 0xFF) as u8,
                ((pixel >> 8) & 0xFF) as u8,
            ])?;
        }
    }
    output.flush()?;
    Ok(())
}

// PCM is what I am calling it, however, it is just all of the samples copied
// into a file: https://en.wikipedia.org/wiki/Pulse-code_modulation
fn pcm(output: &mut impl std::io::Write, samples: &[(f32, f32)]) -> std::io::Result<()> {
    for (l, r) in samples.iter() {
        output.write_all(&l.to_le_bytes())?;
        output.write_all(&r.to_le_bytes())?;
    }
    output.flush()?;
    Ok(())
}

fn ffmpeg(fps: usize, sample_rate: usize) -> std::io::Result<()> {
    let fps = &format!("{fps}");
    let sample_rate = &format!("{sample_rate}");
    #[rustfmt::skip]
    std::process::Command::new("ffmpeg")
        .args([
            "-framerate", fps,
            "-i", "data/frames/%d.ppm",
            "-f", "f32le",
            "-ar", sample_rate,
            "-ac", "2",
            "-i", "data/audio.pcm",
            "-c:v", "libx264",
            "-preset", "medium",
            "-crf", "23",
            "-pix_fmt", "yuv420p",
            "-c:a", "aac",
            "-b:a", "192k",
            "-shortest",
            "out.mp4",
        ])
            .spawn()
            .unwrap()
            .wait()?;
    Ok(())
}
