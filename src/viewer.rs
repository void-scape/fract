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
    #[cfg(target_arch = "wasm32")]
    pipeline_builder: Option<crate::pipeline::PipelineBuilder>,
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
            #[cfg(target_arch = "wasm32")]
            pipeline_builder: None,
        }
    }
}

fn handle_input(
    glazer::PlatformInput {
        window,
        memory,
        input,
        ..
    }: glazer::PlatformInput<Memory>,
) {
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
            // let scale = window.scale_factor();
            // memory.cursor_y /= scale;
            // memory.cursor_x /= scale;
        }
        glazer::Input::Window(WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button,
            ..
        }) => {
            let Some(pipeline) = memory.pipeline.as_mut() else {
                return;
            };

            let physical_size = window.inner_size();
            let w = Float::from(physical_size.width);
            let h = Float::from(physical_size.height);

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

    #[cfg(target_arch = "wasm32")]
    {
        if memory.pipeline.is_none() {
            let builder = memory.pipeline_builder.get_or_insert_with(|| {
                crate::pipeline::PipelineBuilder::new(window, memory.config.clone())
            });
            if builder.poll() {
                memory.pipeline = Some(memory.pipeline_builder.take().unwrap().build());
                reparent_canvas("fract");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    memory
        .pipeline
        .get_or_insert_with(|| Pipeline::new(Some(window), memory.config.clone(), None));

    if let Some(pipeline) = &mut memory.pipeline {
        pipeline.force_step_mandelbrot(memory.config.iterations);
    }
}

#[cfg(target_arch = "wasm32")]
fn reparent_canvas(container_id: &str) {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.query_selector("canvas").unwrap().unwrap();
    let container = document.get_element_by_id(container_id).unwrap();
    container.append_child(&canvas).unwrap();
    let html_canvas = canvas.dyn_into::<web_sys::HtmlCanvasElement>().unwrap();
    html_canvas.style().set_property("width", "100%").unwrap();
    html_canvas.style().set_property("height", "100%").unwrap();
}
