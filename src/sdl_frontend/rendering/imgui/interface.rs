use crate::rendering::imgui::state::{State, DebugState};
use imgui::*;
use sdl2::keyboard::Scancode;
use rustyboi_core::hardware::ppu::palette::RGB;

pub fn create_main_menu_bar(state: &mut State, ui: &Ui) {
    ui.main_menu_bar(|| {
        ui.menu(im_str!("Debug"), true, || {
            if MenuItem::new(im_str!("ImGui Metrics"))
                .build_with_ref(ui, &mut state.show_metrics) {
            }
        });
        ui.menu(im_str!("Views"), true, || {
            if MenuItem::new(im_str!("Palette View"))
                .shortcut(im_str!("Ctrl+P"))
                .build_with_ref(ui, &mut state.palette_window) {
            }
        });
        main_menu_shortcuts(state, ui);
    })
}

#[inline(always)]
pub fn main_menu_shortcuts(state: &mut State, ui: &Ui) {
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::P as u32) {
        state.palette_window = !state.palette_window;
    }
}

pub fn show_metrics(state: &mut State, ui: &Ui) {
    if state.show_metrics {
        ui.show_metrics_window(&mut state.show_metrics);
    }
}

pub fn show_palette_view(state: &mut State, ui: &Ui, debug_state: &mut DebugState) {
    if state.palette_window {
        if let Some(palette) = debug_state.palette.as_ref() {
            Window::new(im_str!("Palette View"))
                .size(size_a(ui, [200.0, 100.0]), Condition::Appearing)
                .opened(&mut state.palette_window)
                .build(ui, || {
                    ui.text("Hello World!");
                    ColorButton::new(im_str!("color_button"), rgb_to_imgui(palette.dmg_bg_palette[0]))
                        .build(&ui);
                })
        }
    }
}

fn rgb_to_imgui(rgb: RGB) -> [f32; 4] {
    let mut result = [1.0; 4];
    let to_f32 = |c| c as f32 / 255.0;
    result = [to_f32(rgb.0), to_f32(rgb.1), to_f32(rgb.2), 1.0];

    result
}

fn size(ui: &Ui, size: f32) -> f32 {
    size * ui.current_font_size()
}

fn size_a(ui: &Ui, mut sizes: [f32; 2]) -> [f32; 2] {
    sizes.iter_mut().map(|s| size(ui, *s));
    sizes
}
