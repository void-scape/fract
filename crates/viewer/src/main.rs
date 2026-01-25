use compute::PRECISION;
use glazer::winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use rug::{
    Float,
    float::Round,
    ops::{AddAssignRound, MulAssignRound},
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
            button,
            ..
        }) => {
            let factor = match button {
                MouseButton::Left => 0.5,
                MouseButton::Right => 2.0,
                _ => return,
            };

            let zs = match button {
                MouseButton::Left => 1.0,
                MouseButton::Right => -1.0,
                _ => return,
            };

            let w = compute::WIDTH as f64;
            let h = compute::HEIGHT as f64;
            let aspect = w / h;

            let dx = ((memory.cursor_x / w) * 2.0 - 1.0) * aspect * zs;
            let dy = ((memory.cursor_y / h) * 2.0 - 1.0) * zs;
            let dcx = Float::with_val(PRECISION, &memory.zoom * dx);
            let dcy = Float::with_val(PRECISION, &memory.zoom * dy);
            memory.cx.add_assign_round(dcx, Round::Nearest);
            memory.cy.add_assign_round(dcy, Round::Nearest);
            memory.zoom.mul_assign_round(factor, Round::Nearest);
        }
        _ => {}
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
