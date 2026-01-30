use crate::{config::Config, pipeline::Pipeline};
use glazer::winit::{
    event::{DeviceEvent, ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};
use malachite::base::num::basic::traits::{NegativeOne, One, OneHalf, Two};
use malachite_float::Float;

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
                let factor = match button {
                    MouseButton::Left => Float::ONE_HALF,
                    MouseButton::Right => Float::TWO,
                    _ => return,
                };
                let zs = match button {
                    MouseButton::Left => Float::ONE,
                    MouseButton::Right => Float::NEGATIVE_ONE,
                    _ => return,
                };

                let cx = Float::from(memory.cursor_x);
                let cy = Float::from(memory.cursor_y);
                let w = Float::from(memory.config.width);
                let h = Float::from(memory.config.height);
                let one = Float::ONE;
                let two = Float::TWO;

                let dx = (&cx / &w * &two - &one) * w / &h * &zs;
                let dy = (&cy / &h * &two - &one) * zs * Float::NEGATIVE_ONE;

                *x += &*z * &dx;
                *y += &*z * &dy;
                *z *= factor;
            });
        }
        glazer::Input::Device(DeviceEvent::MouseWheel { delta }) => match delta {
            MouseScrollDelta::PixelDelta(delta) => {
                let Some(pipeline) = memory.pipeline.as_mut() else {
                    return;
                };

                pipeline.write_position(|_, _, z| {
                    let sensitivity = 0.005;
                    *z *= Float::from((-delta.y * sensitivity).exp());
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
