use nanoserde::{DeJson, SerJson};
use rustyboi_core::hardware::ppu::debugging_features::PaletteDebugInfo;
use rustyboi_core::emulator::EmulatorMode;
use std::time::Duration;

#[derive(Default, Debug, Copy, Clone, DeJson, SerJson)]
pub struct State {
    pub show_metrics: bool,
    pub show_settings: bool,
    pub palette_window: bool,
    pub tile_display: bool,
    pub execution_log: bool,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default()
    }
}

#[derive(Default, Debug, Clone)]
pub struct DebugState {
    pub current_emu_mode: EmulatorMode,
    pub palette: PaletteDebugInfo,
    pub notification: Notification,
}

#[derive(Default, Debug, Clone)]
pub struct Notification {
    pub remaining_animation_duration: f32,
    pub animation_per_second: f32,
    pub message: &'static str,
}

impl Notification {
    pub fn new(message: &'static str) -> Notification {
        Notification {
            remaining_animation_duration: 1.0,
            animation_per_second: 0.6,
            message
        }
    }

    pub fn with_duration(message: &'static str, duration: Duration) -> Notification {
        Notification {
            remaining_animation_duration: 1.0,
            animation_per_second: 1.0 / duration.as_secs_f32(),
            message
        }
    }
}