use nanoserde::{DeJson, SerJson};
use rustyboi_core::hardware::ppu::debugging_features::PaletteDebugInfo;
use rustyboi_core::emulator::EmulatorMode;
use std::time::{Duration, Instant};
use crate::rendering::imgui::animate::{FadeAnimation, formulas::Quadratic};
use imgui::Ui;
use crate::rendering::imgui::animate::formulas::ParametricBlend;

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
    pub animation: FadeAnimation<Quadratic>,
    pub message: &'static str,
}

impl Notification {
    pub fn new(message: &'static str, ui: &Ui) -> Notification {
        Notification {
            animation: FadeAnimation::new(ui, Duration::from_millis(2000)),
            message
        }
    }

    pub fn with_duration(message: &'static str, duration: Duration, ui: &Ui) -> Notification {
        Notification {
            animation: FadeAnimation::new(ui, duration),
            message
        }
    }
}