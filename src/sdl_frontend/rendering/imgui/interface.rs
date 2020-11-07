use imgui::*;
use crate::rendering::imgui::state::State;
use sdl2::keyboard::Scancode;

pub fn create_main_menu_bar(state: &mut State, ui: &Ui) {
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
        main_menu_shortcuts(state, ui);
    })
}

#[inline(always)]
pub fn main_menu_shortcuts(state: &mut State, ui: &Ui) {
    if ui.io().key_ctrl && ui.is_key_pressed(Scancode::P as u32){
        state.palette_window = !state.palette_window;
    }
}

pub fn show_metrics(state: &mut State, ui: &Ui) {
    if state.show_metrics {
        ui.show_metrics_window(&mut state.show_metrics);
    }
}

pub fn show_palette_view(state: &mut State, ui: &Ui) {
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