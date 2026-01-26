use crate::PRECISION;
use glazer::winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use rug::{
    Float,
    float::Round,
    ops::{AddAssignRound, MulAssignRound, SubAssignRound},
};
use tint::Sbgr;

pub fn run(memory: Memory) {
    let width = memory.width;
    let height = memory.height;
    glazer::run(memory, width, height, handle_input, update_and_render, None);
}

pub struct Memory {
    pub zoom: Float,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub cx: Float,
    pub cy: Float,
    pub iterations: usize,
    pub width: usize,
    pub height: usize,
    pub palette: Vec<Sbgr>,
    pub pipeline: Option<crate::pipeline::Pipeline>,
}

fn handle_input(glazer::PlatformInput { memory, input, .. }: glazer::PlatformInput<Memory>) {
    let w = memory.width as f64;
    let h = memory.height as f64;
    let aspect = w / h;

    match input {
        glazer::Input::Window(WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state: glazer::winit::event::ElementState::Pressed,
                    repeat: false,
                    ..
                },
            ..
        }) => match key {
            KeyCode::Escape => {
                println!("x = \"{}\"", memory.cx.to_string_radix(10, None));
                println!("y = \"{}\"", memory.cy.to_string_radix(10, None));
                println!("zoom = \"{}\"\n", memory.zoom.to_string_radix(10, None));
                std::process::exit(0);
            }
            KeyCode::KeyP => {
                println!("x = \"{}\"", memory.cx.to_string_radix(10, None));
                println!("y = \"{}\"", memory.cy.to_string_radix(10, None));
                println!("zoom = \"{}\"\n", memory.zoom.to_string_radix(10, None));
            }
            _ => {}
        },
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

            let dx = ((memory.cursor_x / w) * 2.0 - 1.0) * aspect * zs;
            let dy = ((memory.cursor_y / h) * 2.0 - 1.0) * -zs;
            let dcx = Float::with_val(PRECISION, &memory.zoom * dx);
            let dcy = Float::with_val(PRECISION, &memory.zoom * dy);
            memory.cx.add_assign_round(dcx, Round::Nearest);
            memory.cy.add_assign_round(dcy, Round::Nearest);
            memory.zoom.mul_assign_round(factor, Round::Nearest);
        }
        glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
            MouseScrollDelta::PixelDelta(delta) => {
                let delta =
                    delta.y.signum() * (delta.x * delta.x + delta.y * delta.y).sqrt() / h * 10.0;
                let zd = Float::with_val(PRECISION, delta * &memory.zoom);

                let dx = Float::with_val(PRECISION, ((memory.cursor_x / w) * 2.0 - 1.0) * aspect);
                let dy = Float::with_val(PRECISION, ((memory.cursor_y / h) * 2.0 - 1.0) * -1.0);
                memory.cx.add_assign_round(&dx * &zd, Round::Nearest);
                memory.cy.add_assign_round(&dy * &zd, Round::Nearest);
                memory.zoom.sub_assign_round(&zd, Round::Nearest);
            }
            _ => unimplemented!(),
        },
        _ => {}
    }
}

fn update_and_render(
    glazer::PlatformUpdate { window, memory, .. }: glazer::PlatformUpdate<Memory>,
) {
    window.set_resizable(false);
    window.set_title("Mandelbrot Set");

    let pipeline = memory.pipeline.get_or_insert_with(|| {
        crate::pipeline::create_pipeline(
            Some(window),
            &memory.palette,
            memory.width,
            memory.height,
            false,
        )
    });
    crate::pipeline::compute_mandelbrot(
        pipeline,
        memory.iterations,
        &memory.zoom,
        &memory.cx,
        &memory.cy,
    );
}
