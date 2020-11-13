use crate::rendering::imgui::state::{GuiState, Notification, DebugState};
use imgui::*;
use nanoserde::*;
use crate::rendering::imgui::interface::{show_help_marker, size_a, size, ImguiColour};
use crate::GLOBAL_APP_STATE;
use std::str::FromStr;
use std::time::Duration;
use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};
use crate::state::{DisplayColourDTO, DisplayColourConfigurable};

const SUB_MENUS: [&str; 2] = ["General", "Display"];

#[derive(Default, Debug, Clone, DeJson, SerJson)]
pub struct SettingScreenState {
    current_item: String,
    general_selected: bool,
    display_selected: bool,
}

pub fn render_settings(state: &mut GuiState, ui: &Ui, debug_state: &mut DebugState) {
    if !state.show_settings {
        return;
    }
    let mut window_open = true;
    Window::new(im_str!("Settings"))
        .size(size_a(ui, [30.0, 40.0]), Condition::Appearing)
        .opened(&mut window_open)
        .build(ui, || {
            let parent_window = ui.window_size();
            ui.columns(2, im_str!("settings_columns"), true);
            ui.set_current_column_width(parent_window[0] / 3.0 + size(ui, 0.5));
            ChildWindow::new(im_str!("Setting Selection"))
                .size([parent_window[0] / 3.0, parent_window[1] * 0.90])
                .movable(false)
                .border(true)
                .build(ui, || {
                    ui.set_window_font_scale(1.2);
                    create_selectables(ui, state);
                    ui.set_window_font_scale(1.0);
                });
            ui.next_column();
            create_settings(ui, state, debug_state);
        });
    state.show_settings = window_open;
}

fn create_settings(ui: &Ui, state: &mut GuiState, debug_state: &mut DebugState) {
    match state.setting_state.current_item.as_str() {
        "General" => {
            ui.text("Fast Forward Speed:");
            ui.same_line(0.0);
            show_help_marker(ui, "Sets the emulation speed while pressing LSHIFT.\
            \nValues in the integer range of [1, 100] are allowed.");
            ui.same_line_with_spacing(0.0, size(ui, 2.0));
            let mut input = ImString::new(format!("{}", GLOBAL_APP_STATE.lock().unwrap().fast_forward_rate));
            if ui.input_text(im_str!("Fast Forward Speed"), &mut input)
                .chars_decimal(true)
                .allow_tab_input(false)
                .auto_select_all(true)
                .build() {
                if let Ok(new_multiplier) = u64::from_str(input.as_ref()) {
                    if (1..=100).contains(&new_multiplier) {
                        GLOBAL_APP_STATE.lock().unwrap().fast_forward_rate = new_multiplier;
                    } else {
                        debug_state.notification = Notification::with_duration("Value too large (max 40).\nRun unbounded instead", Duration::from_millis(300), ui);
                    }
                } else {
                    debug_state.notification = Notification::new("Only integers are valid!", ui);
                }
            }
        },
        "Display" => {
            let mut global_state = GLOBAL_APP_STATE.lock().unwrap();
            create_display_colour_picker(ui, "Background Palette:", &mut global_state.custom_display_colour.dmg_bg_colour, "Bg");
            create_display_colour_picker(ui, "Sprite Palette 0:", &mut global_state.custom_display_colour.dmg_sprite_colour_0, "Sp0");
            create_display_colour_picker(ui, "Sprite Palette 1:", &mut global_state.custom_display_colour.dmg_sprite_colour_1, "Sp1");
            ui.text("Reset colours to default:");
            ui.same_line(0.0);
            if ui.button(im_str!("Reset"), size_a(ui, [4.0, 1.2])) {
                global_state.custom_display_colour = DisplayColourConfigurable::default();
            }
        }
        _ => {}
    }
    ui.set_window_font_scale(1.0);
}

fn create_display_colour_picker(ui: &Ui, title: impl AsRef<str>, linked_display: &mut DisplayColourDTO, suffix: impl AsRef<str>) {
    ui.text(title.as_ref());
    ui.same_line(0.0);
    ui.set_cursor_pos([ui.window_size()[0] - size(ui, 8.0), ui.cursor_pos()[1]]);
    create_picker(ui, format!("White {}", suffix.as_ref()), &mut linked_display.white);
    ui.same_line(0.0);
    create_picker(ui, format!("Light Grey {}", suffix.as_ref()), &mut linked_display.light_grey);
    ui.same_line(0.0);
    create_picker(ui, format!("Dark Grey {}", suffix.as_ref()), &mut linked_display.dark_grey);
    ui.same_line(0.0);
    create_picker(ui, format!("Black {}", suffix.as_ref()), &mut linked_display.black);
}

fn create_picker(ui: &Ui, title: impl AsRef<str>, linked_rgb: &mut (u8, u8, u8)) {
    let mut editable_colour = linked_rgb.into_f();
    if ColorEdit::new(&im_str!("{}", title.as_ref()), &mut editable_colour)
        .format(ColorFormat::U8)
        .label(false)
        .alpha(false)
        .alpha_bar(false)
        .options(true)
        .input_mode(ColorEditInputMode::RGB)
        .display_mode(ColorEditDisplayMode::RGB)
        .picker(true)
        .inputs(false)
        .build(ui) {
        *linked_rgb = f32_to_rgb(editable_colour);
    }
}

fn create_selectables(ui: &Ui, state: &mut GuiState) {
    for  menu in SUB_MENUS.iter() {
        let is_selected = state.setting_state.current_item == *menu;
        if Selectable::new(&im_str!("{}", menu))
            .selected(is_selected)
            .build(ui) {
            state.setting_state.current_item = menu.to_string();
        }
    }
}

fn f32_to_rgb(input: [f32; 4]) -> (u8, u8, u8) {
    ((input[0] * 255.) as u8, (input[1] * 255.) as u8, (input[2] * 255.) as u8)
}