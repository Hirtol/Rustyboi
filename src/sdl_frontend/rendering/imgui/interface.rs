use imgui::*;
use sdl2::keyboard::Scancode;

use rustyboi_core::hardware::ppu::palette::RGB;

use crate::rendering::imgui::state::{DebugState, State, Notification};
use std::time::Duration;

pub fn create_main_menu_bar(state: &mut State, ui: &Ui) {
    ui.main_menu_bar(|| {
        ui.menu(im_str!("Debug"), true, || {
            if MenuItem::new(im_str!("ImGui Metrics"))
                .build_with_ref(ui, &mut state.show_metrics) {}
            MenuItem::new(im_str!("Settings"))
                .shortcut(im_str!("Ctrl+S"))
                .build_with_ref(ui, &mut state.show_settings);
        });
        ui.menu(im_str!("Views"), true, || {
            MenuItem::new(im_str!("Execution"))
                .shortcut(im_str!("Ctrl+E"))
                .build_with_ref(ui, &mut state.execution_log);
            MenuItem::new(im_str!("Palette View"))
                .shortcut(im_str!("Ctrl+P"))
                .build_with_ref(ui, &mut state.palette_window);
            MenuItem::new(im_str!("VRAM View"))
                .shortcut(im_str!("Ctrl+Q"))
                .build_with_ref(ui, &mut state.tile_display);

        });
        main_menu_shortcuts(state, ui);
    })
}

pub fn show_notification(debug: &mut DebugState, ui: &Ui) {
    if debug.notification.animation.finished() {
        return;
    }
    //TODO: Change to const if f32 in const functions ever gets stabilised.
    let alert_colour: [f32; 4] = rgb_to_f32(51, 51, 153);
    let display_size = ui.io().display_size;
    let window_width = size(ui, 10f32.max(display_size[0] / 180.));
    let window_height =  size(ui, 7f32.max(display_size[0] / 270.));
    let window_pos = [display_size[0] - window_width - size(ui, 3.), display_size[1] - window_height - size(ui, 3.)];
    let style = ui.push_style_var(StyleVar::Alpha(debug.notification.animation.progress()));
    Window::new(im_str!("Notification"))
        .position(window_pos, Condition::Always)
        .size([window_width, window_height], Condition::Always)
        .title_bar(false)
        .resizable(false)
        .movable(false)
        .focus_on_appearing(false)
        .save_settings(false)
        .build(ui, || {
            ui.set_window_font_scale(1.5);
            ui.text_colored(alert_colour, "Alert");
            ui.set_window_font_scale(1.0);
            ui.separator();
            ui.text_wrapped(&im_str!("{}", debug.notification.message));
            if !ui.is_window_hovered() {
                debug.notification.animation.progress_animation(ui);
            } else {
                debug.notification.animation.partial_reset(ui);
            }
        });
    style.pop(ui);
}

#[inline(always)]
pub fn main_menu_shortcuts(state: &mut State, ui: &Ui) {
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::S as u32) {
        state.show_settings = !state.show_settings;
    }
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::E as u32) {
        state.execution_log = !state.execution_log;
    }
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::P as u32) {
        state.palette_window = !state.palette_window;
    }
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::Q as u32) {
        state.tile_display = !state.tile_display;
    }
}

pub fn show_metrics(state: &mut State, ui: &Ui) {
    if state.show_metrics {
        ui.show_metrics_window(&mut state.show_metrics);
    }
}

pub fn show_palette_view(state: &mut State, ui: &Ui, debug_state: &mut DebugState) {
    if state.palette_window {
        Window::new(im_str!("Palette View"))
            .size(size_a(ui, [25.0, 17.5]), Condition::Appearing)
            .resizable(false)
            .opened(&mut state.palette_window)
            .build(ui, || {
                ui.columns(2, im_str!("Palettes"), true);
                ui.text("Background Palettes:");
                show_palettes_column(ui, &mut debug_state.notification, &debug_state.palette.bg_palette, "BG Colour ");
                ui.next_column();
                ui.text("Sprite Palettes:");
                ui.same_line(0.0);
                show_help_marker(ui, "Hover over a palette colour to see its values, left click to copy");
                show_palettes_column(ui, &mut debug_state.notification, &debug_state.palette.sprite_palette, "Sprite Colour ");
            });
    }
}

#[inline(always)]
fn show_palettes_column(ui: &Ui, notification: &mut Notification, palettes: &Vec<[RGB; 4]>, name_prefix: &str) {
    //TODO: Figure out why we get a stack overflow if we don't inline this?!
    for (index, c_palette) in palettes.iter().enumerate() {
        move_hori_cursor(ui, 2.0);
        for (i, rgb) in c_palette.iter().enumerate() {
            if ColorButton::new(&im_str!("{} {}", name_prefix, index*4 + i), rgb_to_imgui(rgb))
                .alpha(false)
                .build(&ui) {
                ui.set_clipboard_text(&im_str!("{:?}", rgb));
                *notification = Notification::with_duration("Copied colour to clipboard!", Duration::from_millis(2000), ui);
            }
            if i != 3 {
                ui.same_line(0.0);
            }
        }
    }
}

fn rgb_to_imgui(rgb: &RGB) -> [f32; 4] {
    rgb_to_f32(rgb.0, rgb.1, rgb.2)
}

#[inline]
/// Taken from the demo example, as it's rather helpful.
fn show_help_marker(ui: &Ui, desc: &str) {
    ui.text_disabled(im_str!("(?)"));
    if ui.is_item_hovered() {
        ui.tooltip(|| {
            ui.text(desc);
        });
    }
}

#[inline(always)]
fn move_hori_cursor(ui: &Ui, to_move: f32) {
    let current_cursor = ui.cursor_pos();
    ui.set_cursor_pos([current_cursor[0] + size(ui, to_move), current_cursor[1]]);
}

#[inline(always)]
fn size(ui: &Ui, size: f32) -> f32 {
    size * ui.current_font_size()
}

#[inline(always)]
fn size_a(ui: &Ui, mut sizes: [f32; 2]) -> [f32; 2] {
    for siz in &mut sizes {
        *siz = size(ui, *siz);
    }
    sizes
}

fn rgb_to_f32(red: u8, green: u8, blue: u8) -> [f32; 4] {
    [red as f32 /255., green as f32 /255., blue as f32 /255., 1.0]
}
