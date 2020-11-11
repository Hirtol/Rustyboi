use crate::rendering::imgui::state::{GuiState, Notification, DebugState};
use imgui::*;
use nanoserde::*;
use crate::rendering::imgui::interface::{show_help_marker, size_a, size};
use crate::GLOBAL_APP_STATE;
use std::str::FromStr;
use std::time::Duration;

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
            ChildWindow::new(im_str!("Setting Selection"))
                .size([parent_window[0] / 3.0, parent_window[1] * 0.90])
                .movable(false)
                .border(true)
                .build(ui, || {
                    ui.set_window_font_scale(1.2);
                    create_selectables(ui, state);
                    ui.set_window_font_scale(1.0);
                });
            ui.same_line(0.0);
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
            \nValues in the integer range of [1, 40] are allowed.");
            ui.same_line_with_spacing(0.0, size(ui, 2.0));
            let mut input = ImString::new(format!("{}", GLOBAL_APP_STATE.lock().unwrap().fast_forward_rate));
            if ui.input_text(im_str!("Fast Forward Speed"), &mut input)
                .chars_decimal(true)
                .allow_tab_input(false)
                .auto_select_all(true)
                .build() {
                if let Ok(new_multiplier) = u64::from_str(input.as_ref()) {
                    if new_multiplier <= 40 {
                        GLOBAL_APP_STATE.lock().unwrap().fast_forward_rate = new_multiplier;
                    } else {
                        debug_state.notification = Notification::with_duration("Value too large (max 40).\nRun unbounded instead", Duration::from_millis(300),ui);
                    }
                } else {
                    debug_state.notification = Notification::new("Only integers are valid!", ui);
                }
            }
        },
        _ => {}
    }
    ui.set_window_font_scale(1.0);
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