mod game;
mod level;
mod raycaster;
mod audio;
mod fonts;
mod sprites;

use crate::game::Game;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 400;

fn main() -> anyhow::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Pacman 3D - Raycaster (Rust)")
        .with_inner_size(LogicalSize::new(WIDTH as f64, HEIGHT as f64))
        .with_resizable(false)
        .build(&event_loop)
        .unwrap();

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap();

    let mut game = Game::new(WIDTH as i32, HEIGHT as i32)?;

    // Intentar capturar el cursor (rotación con mouse horizontal)
    let _ = window.set_cursor_grab(winit::window::CursorGrabMode::Confined);
    window.set_cursor_visible(false);

    let mut last_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::DeviceEvent { event, .. } => {
                if let DeviceEvent::MouseMotion { delta: (dx, _dy) } = event {
                    game.on_mouse_delta(dx as f32);
                }
            }
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    } => {
                        let pressed = state == ElementState::Pressed;
                        if pressed && keycode == VirtualKeyCode::Escape {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                        game.on_key(keycode, pressed);
                    }
                    _ => {}
                }
            }
            Event::MainEventsCleared => {
                // Timing
                let now = std::time::Instant::now();
                let dt = (now - last_time).as_secs_f32();
                last_time = now;

                game.update(dt);

                // Render
                let frame = pixels.frame_mut();
                game.render(frame, WIDTH as i32, HEIGHT as i32);

                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            _ => {}
        }
    });
}