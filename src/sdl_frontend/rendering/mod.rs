use std::sync::Arc;
use std::time::Instant;

use sdl2::mouse::MouseState;
use sdl2::render::{Canvas, Texture};
use sdl2::video::{GLContext, GLProfile, Window, WindowPos, FullscreenType};
use sdl2::{EventPump, VideoSubsystem};

use rustyboi_core::hardware::ppu::palette::RGB;
use rustyboi_core::hardware::ppu::{FRAMEBUFFER_SIZE, RESOLUTION_WIDTH};

use crate::rendering::immediate::ImmediateGui;
use sdl::{setup_sdl, transmute_framebuffer};
use rustyboi::storage::FileStorage;
use crate::communication::DebugMessage;

pub mod imgui;
pub mod immediate;
mod sdl;

pub struct Renderer<T>
where
    T: ImmediateGui,
{
    pub sdl_video_system: sdl2::VideoSubsystem,
    pub main_window: Canvas<Window>,
    pub main_texture: Texture,
    pub debug_window: Option<Window>,
    pub immediate_gui: Option<T>,
    /// For SDL we require OpenGL, which uses a Vsync which would block the main thread.
    /// By using this we'll ensure the GUI only renders at the refresh rate of the current monitor.
    last_immediate_frame: Instant,
    gl_context: Option<GLContext>,
    storage: Arc<FileStorage>,
}

impl<T> Renderer<T>
where
    T: ImmediateGui,
{
    pub fn new(sdl_video_system: VideoSubsystem, storage: Arc<FileStorage>) -> anyhow::Result<Self> {
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
            gl_context: None,
            storage,
        })
    }

    /// Closes the debug window, and drops any contexts that were present.
    pub fn close_immediate_gui(&mut self) {
        self.debug_window = None;
        self.immediate_gui = None;
        self.gl_context = None;
    }

    /// Render a new frame in the main window.
    #[inline(always)]
    pub fn render_main_window(&mut self, framebuffer: &[RGB; FRAMEBUFFER_SIZE]) {
        self.main_texture
            .update(None, transmute_framebuffer(framebuffer), RESOLUTION_WIDTH * 3);

        self.main_window.copy(&self.main_texture, None, None);

        self.main_window.present();
    }

    /// Render, if the immediate GUI has been setup, a new frame in the ImGUI.
    /// Can be called very frequently, will never render more than the current monitor's refresh rate.
    #[inline(always)]
    pub fn render_immediate_gui(&mut self, event_pump: &EventPump) -> Option<Vec<DebugMessage>> {
        if let (Some(window), Some(gui)) = (&self.debug_window, &mut self.immediate_gui) {
            let window_flags = window.window_flags();
            if self.last_immediate_frame.elapsed().as_secs_f64()
                >= 1.0 / window.display_mode().unwrap().refresh_rate as f64
                && window_flags & sdl2::sys::SDL_WindowFlags::SDL_WINDOW_HIDDEN as u32 != 1
            {
                let delta = self.last_immediate_frame.elapsed();
                let delta_s = delta.as_nanos() as f32 * 1e-9;
                self.last_immediate_frame = Instant::now();
                // Check whether the main window has SDL_WINDOW_MOUSE_FOCUS, if so, ignore the mouse event.
                let mouse_state = if (window_flags & sdl2::sys::SDL_WindowFlags::SDL_WINDOW_MOUSE_FOCUS as u32) == 0 {
                    MouseState::from_sdl_state(0)
                } else {
                    event_pump.mouse_state()
                };

                gui.prepare_render(delta_s, window, &mouse_state);

                gui.render(window);

                window.gl_swap_window();

                return Some(gui.query_emulator())
            }
        }

        None
    }

    /// Toggle the fullscreen mode of the current main window.
    pub fn toggle_main_window_fullscreen(&mut self) {
        let window = self.main_window.window_mut();
        let current = window.fullscreen_state();
        if current == FullscreenType::Off {
            window.set_fullscreen(FullscreenType::Desktop);
        } else {
            window.set_fullscreen(FullscreenType::Off);
        }
    }

    /// Create a second window and setup the debug context within
    pub fn setup_immediate_gui(&mut self, title: impl AsRef<str>) -> anyhow::Result<()> {
        if let (Some(window), Some(_)) = (&mut self.debug_window, &self.immediate_gui) {
            window.raise();
            Ok(())
        } else {
            // Ensure the video subsystem has created the correct OpenGL context.
            let gl_attr = self.sdl_video_system.gl_attr();
            gl_attr.set_context_profile(GLProfile::Core);
            gl_attr.set_context_version(4, 5);

            self.setup_second_window(title)?;

            // We need the gl_context to not be dropped for the remainder of the program.
            self.gl_context = Some(self.debug_window.as_ref().unwrap().gl_create_context().unwrap());
            gl::load_with(|s| self.sdl_video_system.gl_get_proc_address(s) as _);

            self.immediate_gui = Some(T::new(
                &self.sdl_video_system,
                self.debug_window.as_ref().unwrap(),
                self.storage.clone(),
            ));
            Ok(())
        }
    }

    /// Simple helper function for setting up the immediate gui.
    fn setup_second_window(&mut self, title: impl AsRef<str>) -> anyhow::Result<()> {
        let nr_of_displays = self.sdl_video_system.num_video_displays().unwrap();
        let (x, y) = if nr_of_displays > 1 {
            (
                self.sdl_video_system.display_bounds(1).unwrap().x(),
                self.sdl_video_system.display_bounds(1).unwrap().y(),
            )
        } else {
            (0, 0)
        };

        self.debug_window = Some(
            self.sdl_video_system
                .window(title.as_ref(), 800, 720)
                .position_centered()
                .opengl()
                .resizable()
                .allow_highdpi()
                .hidden()
                .build()?,
        );
        // Center on second screen
        let (w_x, w_y) = self.debug_window.as_ref().unwrap().position();
        self.debug_window
            .as_mut()
            .unwrap()
            .set_position(WindowPos::Positioned(x + w_x), WindowPos::Positioned(y + w_y));
        self.debug_window.as_mut().unwrap().maximize();
        self.debug_window.as_mut().unwrap().show();
        Ok(())
    }
}
