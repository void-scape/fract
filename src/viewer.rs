use crate::{config::Config, pipeline::Pipeline};
use glazer::winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use rug::{
    Float,
    float::Round,
    ops::{AddAssignRound, MulAssignRound, SubAssignRound},
};

pub fn run(memory: Memory) -> ! {
    let width = memory.config.width;
    let height = memory.config.height;
    glazer::run(memory, width, height, handle_input, update_and_render, None);
}

pub struct Memory {
    pipeline: Option<Pipeline>,
    config: Config,
    cursor_x: f64,
    cursor_y: f64,
}

impl Memory {
    pub fn from_config(config: Config) -> Self {
        Self {
            config,
            cursor_x: 0.0,
            cursor_y: 0.0,
            pipeline: None,
        }
    }
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
                if let Some(pipeline) = memory.pipeline.as_mut() {
                    pipeline.read_position(|x, y, z| {
                        println!("x = \"{x}\"");
                        println!("y = \"{y}\"");
                        println!("zoom = \"{z}\"");
                        println!("iterations = {}\n", memory.config.iterations);
                    });
                }
                std::process::exit(0);
            }
            KeyCode::KeyP => {
                if let Some(pipeline) = memory.pipeline.as_mut() {
                    pipeline.read_position(|x, y, z| {
                        println!("x = \"{x}\"");
                        println!("y = \"{y}\"");
                        println!("zoom = \"{z}\"");
                        println!("iterations = {}\n", memory.config.iterations);
                    });
                }
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
            let Some(pipeline) = memory.pipeline.as_mut() else {
                return;
            };

            pipeline.write_position(|x, y, z| {
                let prec = z.prec().max(x.prec()).max(y.prec()) + 10;

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

                let w = memory.config.width as f64;
                let h = memory.config.height as f64;

                let mut dx = Float::with_val(prec, &cx / w);
                dx.mul_assign_round(&two, Round::Nearest);
                dx.sub_assign_round(&one, Round::Nearest);
                dx.mul_assign_round(w / h, Round::Nearest);
                dx.mul_assign_round(&zs, Round::Nearest);

                let mut dy = Float::with_val(prec, &cy / h);
                dy.mul_assign_round(&two, Round::Nearest);
                dy.sub_assign_round(&one, Round::Nearest);
                dy.mul_assign_round(&zs, Round::Nearest);
                dy.mul_assign_round(-1.0, Round::Nearest);

                let dcx = Float::with_val(prec, &*z * &dx);
                let dcy = Float::with_val(prec, &*z * &dy);
                x.add_assign_round(dcx, Round::Nearest);
                y.add_assign_round(dcy, Round::Nearest);
                z.mul_assign_round(factor, Round::Nearest);
            });
        }
        glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
            MouseScrollDelta::PixelDelta(delta) => {
                let Some(pipeline) = memory.pipeline.as_mut() else {
                    return;
                };

                pipeline.write_position(|_, _, z| {
                    let sensitivity = 0.005;
                    z.mul_assign_round((-delta.y * sensitivity).exp(), Round::Nearest);
                });
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

    let pipeline = memory
        .pipeline
        .get_or_insert_with(|| Pipeline::new(Some(window), memory.config.clone(), None));

    pipeline.step_mandelbrot(memory.config.iterations);
    pipeline.present();
}
