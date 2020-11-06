use crate::rendering::immediate::ImmediateGui;
use imgui_sdl2::ImguiSdl2;
use imgui_opengl_renderer::Renderer;
use sdl2::video::{GLProfile, Window, GLContext};
use sdl2::VideoSubsystem;
use sdl2::mouse::MouseState;

pub struct ImguiBoi {
    pub imgui_context: imgui::Context,
    pub opengl_renderer: Renderer,
    pub input_handler: ImguiSdl2,
    gl_context: GLContext,
}

impl ImguiBoi {
    pub fn new(video_subsystem: &sdl2::VideoSubsystem, host_window: &sdl2::video::Window, gl_context: GLContext) -> Self {
        let mut imgui_context = imgui::Context::create();
        let opengl_renderer = imgui_opengl_renderer::Renderer::new(&mut imgui_context, |s| video_subsystem.gl_get_proc_address(s) as _);
        let input_handler = imgui_sdl2::ImguiSdl2::new(&mut imgui_context, host_window);
        Self {
            imgui_context,
            opengl_renderer,
            input_handler,
            gl_context,
        }
    }
}

impl ImmediateGui for ImguiBoi {
    fn new(video_subsystem: &VideoSubsystem, host_window: &Window, gl_context: GLContext) -> Self {
        Self::new(video_subsystem, host_window, gl_context)
    }

    fn query_emulator(&mut self) {
        unimplemented!()
    }

    fn prepare_render(&mut self, delta_time: f32, host_window: &Window, mouse_state: &MouseState) {
        self.input_handler.prepare_frame(self.imgui_context.io_mut(), host_window, mouse_state);
        self.imgui_context.io_mut().delta_time = delta_time;
    }

    fn render(&mut self, host_window: &Window) {
        let ui = self.imgui_context.frame();
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