use compute::PRECISION;
use glazer::winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use rug::{
    Float,
    float::Round,
    ops::{AddAssignRound, SubAssignRound},
};

fn main() {
    glazer::run(
        Memory::default(),
        compute::WIDTH,
        compute::HEIGHT,
        handle_input,
        update_and_render,
        None,
    )
}

struct Memory {
    zoom: Float,
    cursor_x: f64,
    cursor_y: f64,
    cx: Float,
    cy: Float,
    pipeline: Option<compute::pipeline::Pipeline>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            zoom: Float::with_val(PRECISION, 2.0),
            cx: Float::with_val(PRECISION, 0.0),
            cy: Float::with_val(PRECISION, 0.0),
            pipeline: None,
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
            apply_zoom(memory, zoom_delta);
        }
        glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
            MouseScrollDelta::PixelDelta(delta) => {
                let delta = delta.y.signum() * (delta.x * delta.x + delta.y * delta.y).sqrt()
                    / compute::HEIGHT as f64
                    * 10.0;
                let zoom_delta = Float::with_val(PRECISION, delta * &memory.zoom);
                apply_zoom(memory, zoom_delta);
            }
            _ => unimplemented!(),
        },
        _ => {}
    }

    // Claude slop
    fn apply_zoom(memory: &mut Memory, zoom_delta: Float) {
        let w = compute::WIDTH as f64;
        let h = compute::HEIGHT as f64;
        let aspect = w / h;

        // Convert cursor position to normalized coordinates [-1, 1] range
        let norm_x = (memory.cursor_x / w) * 2.0 - 1.0;
        let norm_y = (memory.cursor_y / h) * 2.0 - 1.0;

        // Apply aspect ratio to x coordinate
        let mouse_base_x = norm_x * aspect;
        let mouse_base_y = norm_y;

        // Update zoom
        memory.zoom.sub_assign_round(&zoom_delta, Round::Nearest);

        // Update center position to zoom towards cursor
        let mouse_base_x = Float::with_val(PRECISION, mouse_base_x);
        let mouse_base_y = Float::with_val(PRECISION, mouse_base_y);
        memory
            .cx
            .add_assign_round(&mouse_base_x * &zoom_delta, Round::Nearest);
        memory
            .cy
            .add_assign_round(&mouse_base_y * &zoom_delta, Round::Nearest);
    }
}

fn update_and_render(
    glazer::PlatformUpdate {
        window,
        memory,
        width,
        height,
        ..
    }: glazer::PlatformUpdate<Memory>,
) {
    window.set_resizable(false);
    window.set_title("Mandelbrot Set");

    assert_eq!(width, compute::WIDTH);
    assert_eq!(height, compute::HEIGHT);

    let max_iteration = compute::pipeline::ITERATIONS;
    let pipeline = memory
        .pipeline
        .get_or_insert_with(|| compute::pipeline::create_pipeline(window));
    compute::pipeline::compute_mandelbrot(
        pipeline,
        max_iteration,
        &memory.zoom,
        &memory.cx,
        &memory.cy,
    );
}
