mod assets;
mod engine;
mod input;
mod renderer;
mod scene;
mod time;

use std::sync::Arc;

use engine::Engine;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::WindowBuilder;

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("create event loop");
    let window = Arc::new(
        WindowBuilder::new()
            .with_title("engine2d")
            .build(&event_loop)
            .expect("build window"),
    );

    let mut engine = pollster::block_on(Engine::new(window.clone()));

    event_loop
        .run(move |event, elwt| {
            elwt.set_control_flow(ControlFlow::Poll);

            match event {
                Event::WindowEvent { event, window_id } if window_id == engine.window().id() => {
                    match event {
                        WindowEvent::CloseRequested => elwt.exit(),
                        WindowEvent::Resized(size) => engine.resize(size),
                        WindowEvent::ScaleFactorChanged { .. } => {
                            engine.resize(engine.window().inner_size())
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if let PhysicalKey::Code(code) = event.physical_key {
                                let pressed = event.state == ElementState::Pressed;
                                engine.handle_key(code, pressed);
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            match engine.redraw() {
                                Ok(()) => {}
                                Err(wgpu::SurfaceError::Lost) => engine.resize(engine.window().inner_size()),
                                Err(wgpu::SurfaceError::Outdated) => {}
                                Err(wgpu::SurfaceError::Timeout) => {
                                    log::warn!("Surface timeout")
                                }
                                Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            }
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    engine.window().request_redraw();
                }
                _ => {}
            }
        })
        .expect("run event loop");
}
