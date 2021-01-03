use std::sync::Arc;

use imgui::*;
use imgui_opengl_renderer::Renderer;
use imgui_sdl2::ImguiSdl2;
use sdl2::event::Event;
use sdl2::mouse::MouseState;
use sdl2::VideoSubsystem;

use font::COUSINE_REGULAR_UNCOMPRESSED_DATA;
use crate::data::storage::{FileStorage, Storage};

use crate::data::communication::DebugMessage;
use crate::rendering::imgui::interface::*;
use crate::rendering::imgui::settings::render_settings;
use crate::rendering::imgui::state::{DebugState, GuiState};
use crate::rendering::immediate::ImmediateGui;

mod animate;
mod font;
mod interface;
mod settings;
mod state;

const STATE_FILE_NAME: &str = "debug_config.json";

//TODO: Add dynamic hidpi native support, sadly SDL doesn't have a hidpi query
// function.

pub struct ImguiBoi {
    pub imgui_context: imgui::Context,
    pub opengl_renderer: Renderer,
    pub input_handler: ImguiSdl2,
    gui_state: GuiState,
    debug_state: DebugState,
    storage: Arc<FileStorage>,
}

impl ImguiBoi {
    pub fn new(
        video_subsystem: &sdl2::VideoSubsystem,
        host_window: &sdl2::video::Window,
        storage: Arc<FileStorage>,
    ) -> Self {
        let state: GuiState = storage.get_value(STATE_FILE_NAME).unwrap_or_default();
        let mut imgui_context = imgui::Context::create();
        imgui_context.set_ini_filename(Some(storage.get_dirs().config_dir().join("imgui.ini")));
        let ddpi = video_subsystem.display_dpi(0).unwrap().0;
        let scale = ddpi / 72.0;
        Self::add_fonts(&mut imgui_context, scale);
        imgui_context.style_mut().scale_all_sizes(scale);

        let opengl_renderer =
            imgui_opengl_renderer::Renderer::new(&mut imgui_context, |s| video_subsystem.gl_get_proc_address(s) as _);
        let input_handler = imgui_sdl2::ImguiSdl2::new(&mut imgui_context, host_window);
        ImguiBoi {
            imgui_context,
            opengl_renderer,
            input_handler,
            gui_state: state,
            debug_state: DebugState::default(),
            storage,
        }
    }

    fn add_fonts(imgui_ctx: &mut Context, scale: f32) {
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
    fn new(video_subsystem: &VideoSubsystem, host_window: &sdl2::video::Window, storage: Arc<FileStorage>) -> Self {
        Self::new(video_subsystem, host_window, storage)
    }

    fn query_emulator(&mut self) -> Option<Vec<DebugMessage>> {
        use DebugMessage::*;
        let mut result = Vec::with_capacity(10);

        result.push(Mode(None));

        if self.gui_state.palette_window {
            result.push(Palette(None));
        }

        Some(result)
    }

    fn fulfill_query(&mut self, debug_response: DebugMessage) {
        match debug_response {
            DebugMessage::Palette(info) => self.debug_state.palette = info.unwrap_or_default(),
            DebugMessage::Mode(mode) => self.debug_state.current_emu_mode = mode.unwrap(),
        }
    }

    fn prepare_render(&mut self, delta_time: f32, host_window: &sdl2::video::Window, mouse_state: &MouseState) {
        self.input_handler.prepare_frame(self.imgui_context.io_mut(), host_window, mouse_state);
        self.imgui_context.io_mut().delta_time = delta_time;
    }

    fn render(&mut self, host_window: &sdl2::video::Window) {
        let ui = self.imgui_context.frame();
        ui.show_demo_window(&mut true);

        {
            create_main_menu_bar(&mut self.gui_state, &ui);
            render_notification(&mut self.debug_state, &ui);
            render_metrics(&mut self.gui_state, &ui);
            render_palette_view(&mut self.gui_state, &ui, &mut self.debug_state);
            render_settings(&mut self.gui_state, &ui, &mut self.debug_state);
        }

        // Need to clean the canvas before rendering the next set.
        unsafe {
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        self.input_handler.prepare_render(&ui, host_window);
        self.opengl_renderer.render(ui);
    }

    fn handle_event(&mut self, event: &Event) {
        self.input_handler.handle_event(&mut self.imgui_context, event);
    }
}

impl Drop for ImguiBoi {
    fn drop(&mut self) {
        self.storage.save_value(STATE_FILE_NAME, &self.gui_state).unwrap();
    }
}
