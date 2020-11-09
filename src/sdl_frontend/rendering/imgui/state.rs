use nanoserde::{DeJson, SerJson};
use rustyboi_core::hardware::ppu::debugging_features::PaletteDebugInfo;
use rustyboi_core::emulator::EmulatorMode;

#[derive(Default, Debug, Copy, Clone, DeJson, SerJson)]
pub struct State {
    pub show_metrics: bool,
    pub palette_window: bool,
}

impl State {
    fn reset(&mut self) {
        *self = Self::default()
    }
}

#[derive(Default, Debug, Clone)]
pub struct DebugState {
    pub current_emu_mode: EmulatorMode,
    pub palette: PaletteDebugInfo
}
