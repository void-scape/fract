use glazer::winit::{
    event::{DeviceEvent, KeyEvent, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use std::collections::VecDeque;

fn main() {
    glazer::run(
        Memory::default(),
        fract::WIDTH,
        fract::HEIGHT,
        handle_input,
        update_and_render,
        None,
    )
}

struct Memory {
    fps_window: VecDeque<f32>,
    zoom: f64,
    cursor_x: f64,
    cursor_y: f64,
    cx: f64,
    cy: f64,
    #[cfg(feature = "compute")]
    compute_pipeline: Option<fract::compute::Pipeline>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            fps_window: VecDeque::new(),
            zoom: 1.0,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cx: 0.0,
            cy: 0.0,
            #[cfg(feature = "compute")]
            compute_pipeline: None,
        }
    }
}

fn handle_input(glazer::PlatformInput { memory, input, .. }: glazer::PlatformInput<Memory>) {
    match input {
        glazer::Input::Window(WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                    ..
                },
            ..
        }) => {
            std::process::exit(0);
        }
        glazer::Input::Window(WindowEvent::CursorMoved { position, .. }) => {
            memory.cursor_x = position.x;
            memory.cursor_y = position.y;
        }
        glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
            // Thank you Gemini!
            MouseScrollDelta::PixelDelta(delta) => {
                let zoom_delta = delta.y.signum() * (delta.x * delta.x + delta.y * delta.y).sqrt()
                    / fract::HEIGHT as f64
                    * memory.zoom
                    * 10.0;

                let mouse_base_x =
                    (memory.cursor_x / fract::WIDTH as f64) * fract::MANDELBROT_XRANGE - 2.00;
                let mouse_base_y =
                    (memory.cursor_y / fract::HEIGHT as f64) * fract::MANDELBROT_YRANGE - 1.12;

                memory.zoom -= zoom_delta;

                if memory.zoom <= 0.0 {
                    memory.zoom = 1e-15;
                }

                memory.cx += mouse_base_x * zoom_delta;
                memory.cy += mouse_base_y * zoom_delta;
            }
            _ => unimplemented!(),
        },
        _ => {}
    }
}

fn update_and_render(
    glazer::PlatformUpdate {
        memory,
        frame_buffer,
        width,
        height,
        window,
        delta,
        ..
    }: glazer::PlatformUpdate<Memory>,
) {
    assert_eq!(width, fract::WIDTH);
    assert_eq!(height, fract::HEIGHT);

    let fps_window_size = 50;

    memory.fps_window.push_back(1.0 / delta);
    if memory.fps_window.len() > fps_window_size {
        memory.fps_window.pop_front();
    }

    window.set_title(&format!(
        "fract - {:.2}",
        memory.fps_window.iter().sum::<f32>() / fps_window_size as f32
    ));
    window.set_resizable(false);

    let current_zoom_magnitude = -memory.zoom.log10();
    #[cfg(feature = "compute")]
    let max_iteration = (1000.0 + 500.0 * current_zoom_magnitude.max(0.0)) as usize;
    #[cfg(not(feature = "compute"))]
    let max_iteration = (100.0 + 50.0 * current_zoom_magnitude.max(0.0)) as usize;

    #[cfg(feature = "compute")]
    let pipeline = memory
        .compute_pipeline
        .get_or_insert_with(fract::compute::create_pipeline);

    #[cfg(feature = "compute")]
    fract::compute::compute_mandelbrot(
        pipeline,
        frame_buffer,
        max_iteration,
        memory.zoom,
        memory.cx,
        memory.cy,
    );

    #[cfg(not(feature = "compute"))]
    fract::compute_mandelbrot(
        frame_buffer,
        max_iteration,
        memory.zoom,
        memory.cx,
        memory.cy,
    );
}
