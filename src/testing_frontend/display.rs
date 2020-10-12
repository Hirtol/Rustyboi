use rustyboi_core::hardware::ppu::palette::{DisplayColour, RGB};

pub const TEST_COLOURS: DisplayColour = DisplayColour {
    white: RGB(255, 255, 255),
    light_grey: RGB(123, 255, 49),
    dark_grey: RGB(0, 99, 197),
    black: RGB(0, 0, 0),
};
