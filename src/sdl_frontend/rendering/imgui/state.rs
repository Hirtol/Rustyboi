use crate::rendering::imgui::animate::{formulas::Quadratic, FadeAnimation};
use imgui::Ui;
use nanoserde::{DeJson, SerJson};
use rustyboi_core::gb_emu::GameBoyModel;
use rustyboi_core::hardware::ppu::debugging_features::PaletteDebugInfo;
use std::time::Duration;

use crate::rendering::imgui::settings::SettingScreenState;

#[derive(Default, Debug, Clone, DeJson, SerJson)]
pub struct GuiState {
    pub show_metrics: bool,
    pub show_settings: bool,
    pub palette_window: bool,
    pub tile_display: bool,
    pub execution_log: bool,
    pub setting_state: SettingScreenState,
}

impl GuiState {
    fn reset(&mut self) {
        *self = Self::default()
    }
}

#[derive(Default, Debug, Clone)]
pub struct DebugState {
    pub current_emu_mode: GameBoyModel,
    pub palette: PaletteDebugInfo,
    pub notification: Notification,
}

#[derive(Default, Debug, Clone)]
pub struct Notification {
    pub animation: FadeAnimation<Quadratic>,
    pub message: &'static str,
}

impl Notification {
    pub fn new(message: &'static str, ui: &Ui) -> Notification {
        Notification {
            animation: FadeAnimation::new(ui, Duration::from_millis(2000)),
            message,
        }
    }

    pub fn with_duration(message: &'static str, duration: Duration, ui: &Ui) -> Notification {
        Notification {
            animation: FadeAnimation::new(ui, duration),
            message,
        }
    }
}
