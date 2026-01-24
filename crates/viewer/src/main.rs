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
    #[cfg(not(feature = "software"))]
    pipeline: Option<compute::pipeline::Pipeline>,
    #[cfg(feature = "software")]
    pipeline: compute::software::Pipeline,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            zoom: Float::with_val(PRECISION, 1.0),
            cx: Float::with_val(PRECISION, 0.0),
            cy: Float::with_val(PRECISION, 0.0),
            #[cfg(not(feature = "software"))]
            pipeline: None,
            #[cfg(feature = "software")]
            pipeline: compute::software::Pipeline::default(),
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

    // Gemini slop
    fn apply_zoom(memory: &mut Memory, zoom_delta: Float) {
        let mouse_base_x =
            (memory.cursor_x / compute::WIDTH as f64) * compute::MANDELBROT_XRANGE - 2.00;
        let mouse_base_y =
            (memory.cursor_y / compute::HEIGHT as f64) * compute::MANDELBROT_YRANGE - 1.12;

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
}

fn update_and_render(
    glazer::PlatformUpdate {
        window,
        memory,
        #[cfg(feature = "software")]
        frame_buffer,
        width,
        height,
        ..
    }: glazer::PlatformUpdate<Memory>,
) {
    window.set_resizable(false);
    window.set_title("Mandelbrot Set");

    assert_eq!(width, compute::WIDTH);
    assert_eq!(height, compute::HEIGHT);

    #[cfg(not(feature = "software"))]
    let max_iteration = compute::pipeline::ITERATIONS;
    #[cfg(feature = "software")]
    let current_zoom_magnitude = -memory.zoom.to_f64().log10();
    #[cfg(feature = "software")]
    let max_iteration = (100.0 + 50.0 * current_zoom_magnitude.max(0.0)) as usize;

    #[cfg(not(feature = "software"))]
    let pipeline = memory
        .pipeline
        .get_or_insert_with(|| compute::pipeline::create_pipeline(window));

    #[cfg(not(feature = "software"))]
    compute::pipeline::compute_mandelbrot(
        pipeline,
        max_iteration,
        &memory.zoom,
        &memory.cx,
        &memory.cy,
    );

    #[cfg(feature = "software")]
    compute::software::compute_mandelbrot(
        &mut memory.pipeline,
        frame_buffer,
        max_iteration,
        &memory.zoom,
        &memory.cx,
        &memory.cy,
    );
}
