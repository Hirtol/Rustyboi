use sdl2::render::{Canvas, Texture};
use sdl2::video::{Window, GLProfile};

use crate::rendering::immediate::ImmediateGui;
use rustyboi_core::hardware::ppu::palette::RGB;
use rustyboi_core::hardware::ppu::FRAMEBUFFER_SIZE;
use crate::sdl::{fill_texture_and_copy, setup_sdl};
use std::time::{Instant, Duration};
use sdl2::mouse::MouseState;
use sdl2::{VideoSubsystem, EventPump};

pub mod immediate;
pub mod imgui;

pub struct Renderer<T>
    where T: ImmediateGui {
    pub sdl_video_system: sdl2::VideoSubsystem,
    pub main_window: Canvas<Window>,
    pub main_texture: Texture,
    pub debug_window: Option<Window>,
    pub immediate_gui: Option<T>,
    /// For SDL we require OpenGL, which uses a Vsync which would block the main thread.
    /// By using this we'll ensure the GUI only renders at the resolution of the current monitor
    last_immediate_frame: Instant,
}

impl<T> Renderer<T>
    where T: ImmediateGui {
    pub fn new(sdl_video_system: VideoSubsystem) -> anyhow::Result<Self> {
        let mut main_window = sdl_video_system
            .window("RustyBoi", 800, 720)
            .position_centered()
            .resizable()
            .allow_highdpi()
            .build()?
            .into_canvas()
            .accelerated()
            .build()?;
        let main_texture = setup_sdl(&mut main_window);

        Ok(Renderer {
            sdl_video_system,
            main_window,
            main_texture,
            debug_window: None,
            immediate_gui: None,
            last_immediate_frame: Instant::now(),
        })
    }

    pub fn render_main_window(&mut self, framebuffer: &[RGB; FRAMEBUFFER_SIZE]) {
        fill_texture_and_copy(
            &mut self.main_window,
            &mut self.main_texture,
            framebuffer,
        );

        self.main_window.present();
    }

    pub fn setup_immediate_gui(&mut self, title: impl AsRef<str>) -> anyhow::Result<()>{
        if let (Some(_), Some(_)) = (&self.debug_window, &self.immediate_gui) {
            Ok(())
        } else {
            // Ensure the video subsystem has created the correct OpenGL context.
            let gl_attr = self.sdl_video_system.gl_attr();
            gl_attr.set_context_profile(GLProfile::Core);
            gl_attr.set_context_version(4, 5);

            self.debug_window = Some(self.sdl_video_system.window(title.as_ref(), 800, 720)
                .position_centered()
                .opengl()
                .resizable()
                .allow_highdpi()
                .build()?
            );
            // We need the gl_context to not be dropped for the remainder of the program.
            let _gl_context = self.debug_window.as_ref().unwrap().gl_create_context().unwrap();
            gl::load_with(|s| self.sdl_video_system.gl_get_proc_address(s) as _);

            self.immediate_gui = Some(T::new(&self.sdl_video_system, self.debug_window.as_ref().unwrap(), _gl_context));
            Ok(())
        }
    }

    pub fn render_immediate_gui(&mut self, event_pump: &EventPump) -> anyhow::Result<()>{
        if let (Some(window), Some(gui)) = (&self.debug_window, &mut self.immediate_gui) {
            if self.last_immediate_frame.elapsed() > Duration::from_secs_f64(1.0 / window.display_mode().unwrap().refresh_rate as f64) {
                let delta = self.last_immediate_frame.elapsed();
                let delta_s = (delta.as_secs_f32() + delta.subsec_nanos() as f32) / 1_000_000_000.0;
                self.last_immediate_frame = Instant::now();
                // Check whether the main window has SDL_WINDOW_MOUSE_FOCUS, if so, ignore the mouse event.
                let mouse_state =
                    if (self.main_window.window().window_flags() & 0x0400) != 0 {
                        MouseState::from_sdl_state(0)
                    } else {
                        event_pump.mouse_state()
                    };

                gui.prepare_render(delta_s, window, &mouse_state);

                gui.render(window);

                window.gl_swap_window();
            }
        }

        Ok(())
    }
}