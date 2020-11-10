use nanoserde::{DeJson, SerJson, SerJsonState};
use rustyboi_core::hardware::ppu::palette::DisplayColour;

#[derive(Default, Debug, Copy, Clone)]
/// Struct for non-persistent options during runtime.
pub struct AppEmulatorState {
    /// Whether the emulation is currently paused.
    pub emulator_paused: bool,
    /// Whether we should fast forward at our set fast_forward_rate
    pub fast_forward: bool,
    /// Whether the emulation should run unbounded
    pub unbounded: bool,
    /// Whether the app should exit asap
    pub exit: bool,
    /// Whether we're currently awaiting debug info from the emulation thread.
    pub awaiting_debug: bool,
}

impl AppEmulatorState {
    pub fn reset(&mut self) {
        self.awaiting_debug = false;
        self.emulator_paused = false;
    }
}

#[derive(Debug, Copy, Clone, SerJson, DeJson)]
/// Struct for persistent options.
pub struct AppState {
    /// The speed multiplier to use while fast forwarding.
    pub fast_forward_rate: u64,
    pub custom_display_colour: DisplayColourConfigurable,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            fast_forward_rate: 2,
            custom_display_colour: DisplayColourConfigurable::default(),
        }
    }
}

#[derive(Debug, SerJson, DeJson, Copy, Clone, Default)]
pub struct DisplayColourConfigurable {
    dmg_bg_colour: DisplayColourDTO,
    dmg_sprite_colour_0: DisplayColourDTO,
    dmg_sprite_colour_1: DisplayColourDTO
}

type RGB = (u8, u8, u8);

#[derive(Debug, SerJson, DeJson, Copy, Clone, Default)]
pub struct DisplayColourDTO {
    white: RGB,
    light_grey: RGB,
    dark_grey: RGB,
    black: RGB,
}

impl Into<DisplayColour> for DisplayColourDTO {
    fn into(self) -> DisplayColour {
        DisplayColour {
            white: self.white.into(),
            light_grey: self.light_grey.into(),
            dark_grey: self.dark_grey.into(),
            black: self.black.into()
        }
    }
}