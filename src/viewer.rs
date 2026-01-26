use crate::precision;
use glazer::winit::{
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
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
    pub aspect: Float,
    pub bwidth: Float,
    pub bheight: Float,
    pub rerender: bool,
}

fn handle_input(glazer::PlatformInput { memory, input, .. }: glazer::PlatformInput<Memory>) {
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
            let prec = precision(&memory.zoom);

            let factor = match button {
                MouseButton::Left => 0.5,
                MouseButton::Right => 2.0,
                _ => return,
            };

            let zs = Float::with_val(
                prec,
                match button {
                    MouseButton::Left => 1.0,
                    MouseButton::Right => -1.0,
                    _ => return,
                },
            );

            let cx = Float::with_val(prec, memory.cursor_x);
            let cy = Float::with_val(prec, memory.cursor_y);
            let one = Float::with_val(prec, 1.0);
            let two = Float::with_val(prec, 2.0);

            let mut dx = Float::with_val(prec, &cx / &memory.bwidth);
            dx.mul_assign_round(&two, Round::Nearest);
            dx.sub_assign_round(&one, Round::Nearest);
            dx.mul_assign_round(&memory.aspect, Round::Nearest);
            dx.mul_assign_round(&zs, Round::Nearest);

            let mut dy = Float::with_val(prec, &cy / &memory.bheight);
            dy.mul_assign_round(&two, Round::Nearest);
            dy.sub_assign_round(&one, Round::Nearest);
            dy.mul_assign_round(&zs, Round::Nearest);
            dy.mul_assign_round(-1.0, Round::Nearest);

            let dcx = Float::with_val(prec, &memory.zoom * &dx);
            let dcy = Float::with_val(prec, &memory.zoom * &dy);
            memory.cx.add_assign_round(dcx, Round::Nearest);
            memory.cy.add_assign_round(dcy, Round::Nearest);
            memory.zoom.mul_assign_round(factor, Round::Nearest);

            memory.rerender = true;
        }
        // TODO: high precision
        // glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
        //     MouseScrollDelta::PixelDelta(delta) => {
        //         let delta =
        //             delta.y.signum() * (delta.x * delta.x + delta.y * delta.y).sqrt() / h * 10.0;
        //         let zd = Float::with_val(PRECISION, delta * &memory.zoom);
        //
        //         let dx = Float::with_val(PRECISION, ((memory.cursor_x / w) * 2.0 - 1.0) * aspect);
        //         let dy = Float::with_val(PRECISION, ((memory.cursor_y / h) * 2.0 - 1.0) * -1.0);
        //         memory.cx.add_assign_round(&dx * &zd, Round::Nearest);
        //         memory.cy.add_assign_round(&dy * &zd, Round::Nearest);
        //         memory.zoom.sub_assign_round(&zd, Round::Nearest);
        //     }
        //     _ => unimplemented!(),
        // },
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

    if memory.rerender {
        memory.rerender = false;
        crate::pipeline::compute_mandelbrot(
            pipeline,
            memory.iterations,
            &mut memory.zoom,
            &mut memory.cx,
            &mut memory.cy,
        );
    }
}
