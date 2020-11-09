use imgui::*;
use sdl2::keyboard::Scancode;

use rustyboi_core::hardware::ppu::palette::RGB;

use crate::rendering::imgui::state::{DebugState, State};

pub fn create_main_menu_bar(state: &mut State, ui: &Ui) {
    ui.main_menu_bar(|| {
        ui.menu(im_str!("Debug"), true, || {
            if MenuItem::new(im_str!("ImGui Metrics"))
                .build_with_ref(ui, &mut state.show_metrics) {}
        });
        ui.menu(im_str!("Views"), true, || {
            if MenuItem::new(im_str!("Palette View"))
                .shortcut(im_str!("Ctrl+P"))
                .build_with_ref(ui, &mut state.palette_window) {}
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
        let palette = &debug_state.palette;
        Window::new(im_str!("Palette View"))
            .size(size_a(ui, [500.0, 400.0]), Condition::Appearing)
            .opened(&mut state.palette_window)
            .build(ui, || {
                ui.columns(2, im_str!("Palettes"), true);
                ui.text("Background Palettes:");
                show_palettes_column(ui, &palette.cgb_bg_palette, "BG Colour ");
                ui.next_column();
                ui.text("Sprite Palettes:");
                show_palettes_column(ui, &palette.cgb_sprite_palette, "Sprite Colour ");
            })
    }
}

#[inline(always)]
fn show_palettes_column(ui: &Ui, palettes: &Vec<[RGB; 4]>, name_prefix: &str) {
    //TODO: Figure out why we get a stack overflow if we don't inline this?!
    for (index, c_palette) in palettes.iter().enumerate() {
        move_hori_cursor(ui, 2.0);
        for i in 0..4 {
            ColorButton::new(&im_str!("{} {}", name_prefix, index*4 + i), rgb_to_imgui(c_palette[i]))
                .alpha(false)
                .build(&ui);
            if i != 3 {
                ui.same_line(0.0);
            }
        }
    }
}

fn rgb_to_imgui(rgb: RGB) -> [f32; 4] {
    let mut result = [1.0; 4];
    let to_f32 = |c| c as f32 / 255.0;
    result = [to_f32(rgb.0), to_f32(rgb.1), to_f32(rgb.2), 1.0];

    result
}

#[inline(always)]
fn move_hori_cursor(ui: &Ui, to_move: f32) {
    let current_cursor = ui.cursor_pos();
    ui.set_cursor_pos([current_cursor[0]+size(ui, to_move), current_cursor[1]]);
}

#[inline(always)]
fn size(ui: &Ui, size: f32) -> f32 {
    size * ui.current_font_size()
}

#[inline(always)]
fn size_a(ui: &Ui, mut sizes: [f32; 2]) -> [f32; 2] {
    sizes.iter_mut().map(|s| size(ui, *s));
    sizes
}
