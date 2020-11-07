use std::fs::{read, File};

use imgui::*;
use imgui::internal::RawCast;
use imgui_opengl_renderer::Renderer;
use imgui_sdl2::ImguiSdl2;
use sdl2::mouse::MouseState;
use sdl2::video::{GLContext, GLProfile};
use sdl2::VideoSubsystem;

use font::COUSINE_REGULAR_UNCOMPRESSED_DATA;
use crate::rendering::immediate::ImmediateGui;
use crate::rendering::imgui::state::State;
use std::io::Write;
use nanoserde::{SerJsonState, SerJson};
use std::fs;
use std::sync::Arc;
use rustyboi::storage::{Storage, FileStorage};
use sdl2::keyboard::Scancode;

mod font;
mod state;

const STATE_FILE_NAME: &str = "debug_config.json";

//TODO: Add dynamic hidpi native support, sadly SDL doesn't have a hidpi query
// function.

pub struct ImguiBoi {
    pub imgui_context: imgui::Context,
    pub opengl_renderer: Renderer,
    pub input_handler: ImguiSdl2,
    state: State,
    storage: Arc<FileStorage>
}

impl ImguiBoi {
    pub fn new(video_subsystem: &sdl2::VideoSubsystem, host_window: &sdl2::video::Window, storage: Arc<FileStorage>) -> Self {
        let state: State = storage.get_value(STATE_FILE_NAME).unwrap_or_default();
        let mut imgui_context = imgui::Context::create();
        imgui_context.set_ini_filename(Some(storage.get_dirs().config_dir().join("imgui.ini")));

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
            state,
            storage
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
    fn new(video_subsystem: &VideoSubsystem, host_window: &sdl2::video::Window, storage: Arc<FileStorage>) -> Self {
        Self::new(video_subsystem, host_window, storage)
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
        {
            create_main_menu_bar(&mut self.state, &ui);
            show_metrics(&mut self.state, &ui);
            show_palette_view(&mut self.state, &ui);
        }

        // Need to clean the canvas before rendering the next set.
        unsafe {
            gl::ClearColor(0.2, 0.2, 0.2, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        self.input_handler.prepare_render(&ui, host_window);
        self.opengl_renderer.render(ui);
    }
}

impl Drop for ImguiBoi {
    fn drop(&mut self) {
        self.storage.save_value(STATE_FILE_NAME, &self.state);
    }
}

fn create_main_menu_bar(state: &mut State, ui: &Ui) {
    ui.main_menu_bar(|| {
        ui.menu(im_str!("Debug"), true, || {
            if MenuItem::new(im_str!("ImGui Metrics"))
                .build_with_ref(ui, &mut state.show_metrics) {
            }
        });
        ui.menu(im_str!("Views"), true,|| {
            if MenuItem::new(im_str!("Palette View"))
                .shortcut(im_str!("Ctrl+P"))
                .build_with_ref(ui, &mut state.palette_window) {
            }
        });
        add_main_menu_shortcuts(state, ui);
    })
}

#[inline(always)]
fn add_main_menu_shortcuts(state: &mut State, ui: &Ui) {
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::P as u32){
        state.palette_window = !state.palette_window;
    }
}

fn show_metrics(state: &mut State, ui: &Ui) {
    if state.show_metrics {
        ui.show_metrics_window(&mut state.show_metrics);
    }
}

fn show_palette_view(state: &mut State, ui: &Ui) {
    if state.palette_window {
        Window::new(im_str!("Palette View"))
            .size(size_a(ui, [200.0, 100.0]), Condition::Appearing)
            .opened(&mut state.palette_window)
            .build(ui, || {
                ui.text("Hello World!");
                ColorButton::new(im_str!("color_button"), [1.0, 0.0, 0.0, 1.0])
                    .build(&ui);
            })
    }
}

fn size(ui: &Ui, size: f32) -> f32 {
    size * ui.current_font_size()
}

fn size_a(ui: &Ui, mut sizes: [f32; 2]) -> [f32; 2] {
    sizes.iter_mut().map(|s| size(ui, *s));
    sizes
}
