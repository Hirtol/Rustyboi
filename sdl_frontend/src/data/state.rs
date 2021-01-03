use crate::DEFAULT_DISPLAY_COLOURS;
use nanoserde::{DeJson, SerJson};
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
    pub audio_mute: bool,
    pub audio_volume: f32,
    pub custom_display_colour: DisplayColourConfigurable,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            fast_forward_rate: 2,
            audio_mute: false,
            audio_volume: 0.0,
            custom_display_colour: DisplayColourConfigurable::default(),
        }
    }
}

#[derive(Debug, SerJson, DeJson, Copy, Clone)]
pub struct DisplayColourConfigurable {
    pub dmg_bg_colour: DisplayColourDTO,
    pub dmg_sprite_colour_0: DisplayColourDTO,
    pub dmg_sprite_colour_1: DisplayColourDTO,
}

impl Default for DisplayColourConfigurable {
    fn default() -> Self {
        DisplayColourConfigurable {
            dmg_bg_colour: DEFAULT_DISPLAY_COLOURS.into(),
            dmg_sprite_colour_0: DEFAULT_DISPLAY_COLOURS.into(),
            dmg_sprite_colour_1: DEFAULT_DISPLAY_COLOURS.into(),
        }
    }
}

type RGB = (u8, u8, u8);

#[derive(Debug, SerJson, DeJson, Copy, Clone, Default)]
pub struct DisplayColourDTO {
    pub white: RGB,
    pub light_grey: RGB,
    pub dark_grey: RGB,
    pub black: RGB,
}

impl Into<DisplayColour> for DisplayColourDTO {
    fn into(self) -> DisplayColour {
        DisplayColour {
            white: self.white.into(),
            light_grey: self.light_grey.into(),
            dark_grey: self.dark_grey.into(),
            black: self.black.into(),
        }
    }
}

impl Into<DisplayColourDTO> for DisplayColour {
    fn into(self) -> DisplayColourDTO {
        DisplayColourDTO {
            white: self.white.into(),
            light_grey: self.light_grey.into(),
            dark_grey: self.dark_grey.into(),
            black: self.black.into(),
        }
    }
}
