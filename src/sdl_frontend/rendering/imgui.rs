use std::fs::read;

use imgui::*;
use imgui::internal::RawCast;
use imgui_opengl_renderer::Renderer;
use imgui_sdl2::ImguiSdl2;
use sdl2::mouse::MouseState;
use sdl2::video::{GLContext, GLProfile};
use sdl2::VideoSubsystem;

use crate::rendering::font::COUSINE_REGULAR_UNCOMPRESSED_DATA;
use crate::rendering::immediate::ImmediateGui;

//TODO: Add dynamic hidpi native support, sadly SDL doesn't have a hidpi query
// function.

pub struct ImguiBoi {
    pub imgui_context: imgui::Context,
    pub opengl_renderer: Renderer,
    pub input_handler: ImguiSdl2,
    state: State,
}

impl ImguiBoi {
    pub fn new(video_subsystem: &sdl2::VideoSubsystem, host_window: &sdl2::video::Window) -> Self {
        let mut imgui_context = imgui::Context::create();
        let ddpi = video_subsystem.display_dpi(0).unwrap().0;
        let scale = ddpi / 72.0;
        Self::add_fonts(&mut imgui_context, scale);
        imgui_context.style_mut().scale_all_sizes(scale);
        let opengl_renderer = imgui_opengl_renderer::Renderer::new(&mut imgui_context, |s| video_subsystem.gl_get_proc_address(s) as _);
        let input_handler = imgui_sdl2::ImguiSdl2::new(&mut imgui_context, host_window);
        Self {
            imgui_context,
            opengl_renderer,
            input_handler,
            state: Default::default(),
        }
    }

    fn add_fonts(imgui_ctx: &mut Context, scale:  f32) {
        imgui_ctx.fonts().add_font(&[FontSource::TtfData {
            data: &COUSINE_REGULAR_UNCOMPRESSED_DATA,
            size_pixels: 14.0 * scale,
            config: None,
        }]);
        imgui_ctx.fonts().build_rgba32_texture();
        imgui_ctx.io_mut().font_allow_user_scaling = true;
    }
}

impl ImmediateGui for ImguiBoi {
    fn new(video_subsystem: &VideoSubsystem, host_window: &sdl2::video::Window) -> Self {
        Self::new(video_subsystem, host_window)
    }

    fn query_emulator(&mut self) {
        unimplemented!()
    }

    fn prepare_render(&mut self, delta_time: f32, host_window: &sdl2::video::Window, mouse_state: &MouseState) {
        self.input_handler.prepare_frame(self.imgui_context.io_mut(), host_window, mouse_state);
        self.imgui_context.io_mut().delta_time = delta_time;
    }

    fn render(&mut self, host_window: &sdl2::video::Window) {
        let mut ui = self.imgui_context.frame();
        ui.show_demo_window(&mut true);

        // Need to clean the canvas before rendering the next set.
        unsafe {
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        self.input_handler.prepare_render(&ui, host_window);
        self.opengl_renderer.render(ui);
    }
}

fn size(ui: &Ui, size: f32) -> f32 {
    size * ui.current_font_size()
}

fn size_a(ui: &Ui, mut sizes: [f32; 2]) -> [f32; 2] {
    sizes.iter_mut().map(|s| size(ui, *s));
    sizes
}

#[derive(Default)]
struct State {
    example: u32,
    notify_text: &'static str,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default()
    }
}