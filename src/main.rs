use fract::PRECISION;
use glazer::winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use rug::{
    Float,
    float::Round,
    ops::{AddAssignRound, SubAssignRound},
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
    zoom: Float,
    cursor_x: f64,
    cursor_y: f64,
    cx: Float,
    cy: Float,
    #[cfg(feature = "accelerated")]
    pipeline: Option<fract::accelerated::Pipeline>,
    #[cfg(feature = "software")]
    pipeline: fract::software::Pipeline,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            fps_window: VecDeque::new(),
            cursor_x: 0.0,
            cursor_y: 0.0,
            zoom: Float::with_val(PRECISION, 1.0),
            cx: Float::with_val(PRECISION, 0.0),
            cy: Float::with_val(PRECISION, 0.0),
            #[cfg(feature = "accelerated")]
            pipeline: None,
            #[cfg(feature = "software")]
            pipeline: fract::software::Pipeline::default(),
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
        glazer::Input::Window(WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        }) => {
            let zoom_delta = Float::with_val(PRECISION, &memory.zoom * 0.5);

            let mouse_base_x =
                (memory.cursor_x / fract::WIDTH as f64) * fract::MANDELBROT_XRANGE - 2.00;
            let mouse_base_y =
                (memory.cursor_y / fract::HEIGHT as f64) * fract::MANDELBROT_YRANGE - 1.12;

            memory.zoom.sub_assign_round(&zoom_delta, Round::Nearest);

            let mouse_base_x = Float::with_val(PRECISION, mouse_base_x);
            let mouse_base_y = Float::with_val(PRECISION, mouse_base_y);
            memory
                .cx
                .add_assign_round(&mouse_base_x * &zoom_delta, Round::Nearest);
            memory
                .cy
                .add_assign_round(&mouse_base_y * &zoom_delta, Round::Nearest);
        }
        glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
            // Thank you Gemini!
            MouseScrollDelta::PixelDelta(delta) => {
                let delta = delta.y.signum() * (delta.x * delta.x + delta.y * delta.y).sqrt()
                    / fract::HEIGHT as f64
                    * 10.0;
                let zoom_delta = Float::with_val(PRECISION, delta * &memory.zoom);

                let mouse_base_x =
                    (memory.cursor_x / fract::WIDTH as f64) * fract::MANDELBROT_XRANGE - 2.00;
                let mouse_base_y =
                    (memory.cursor_y / fract::HEIGHT as f64) * fract::MANDELBROT_YRANGE - 1.12;

                memory.zoom.sub_assign_round(&zoom_delta, Round::Nearest);

                let mouse_base_x = Float::with_val(PRECISION, mouse_base_x);
                let mouse_base_y = Float::with_val(PRECISION, mouse_base_y);
                memory
                    .cx
                    .add_assign_round(&mouse_base_x * &zoom_delta, Round::Nearest);
                memory
                    .cy
                    .add_assign_round(&mouse_base_y * &zoom_delta, Round::Nearest);
            }
            _ => unimplemented!(),
        },
        _ => {}
    }
}

fn update_and_render(
    glazer::PlatformUpdate {
        memory,
        #[cfg(feature = "software")]
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

    let fps_window_size = 1;
    memory.fps_window.push_back(1.0 / delta);
    if memory.fps_window.len() > fps_window_size {
        memory.fps_window.pop_front();
    }

    window.set_title(&format!(
        "fract - {:.2}",
        memory.fps_window.iter().sum::<f32>() / fps_window_size as f32
    ));
    window.set_resizable(false);

    #[cfg(feature = "accelerated")]
    let max_iteration = fract::accelerated::ITERATIONS;
    #[cfg(feature = "software")]
    let current_zoom_magnitude = -memory.zoom.to_f64().log10();
    #[cfg(feature = "software")]
    let max_iteration = (100.0 + 50.0 * current_zoom_magnitude.max(0.0)) as usize;

    #[cfg(feature = "accelerated")]
    let pipeline = memory
        .pipeline
        .get_or_insert_with(|| fract::accelerated::create_pipeline(window));

    #[cfg(feature = "accelerated")]
    fract::accelerated::compute_mandelbrot(
        pipeline,
        max_iteration,
        &memory.zoom,
        &memory.cx,
        &memory.cy,
    );

    #[cfg(feature = "software")]
    fract::software::compute_mandelbrot(
        &mut memory.pipeline,
        frame_buffer,
        max_iteration,
        &memory.zoom,
        &memory.cx,
        &memory.cy,
    );
}
