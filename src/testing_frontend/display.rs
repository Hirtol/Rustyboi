use rustyboi_core::hardware::ppu::palette::DmgColor;
use rustyboi_core::hardware::ppu::palette::DmgColor::*;

pub const TEST_COLOURS: DisplayColour = DisplayColour {
    white: RGB(255, 255, 255),
    light_grey: RGB(123, 255, 49),
    dark_grey: RGB(0, 99, 197),
    black: RGB(0, 0, 0),
};

#[derive(Copy, Clone, Debug)]
pub struct RGB(pub u8, pub u8, pub u8);

#[derive(Debug)]
pub struct DisplayColour {
    pub white: RGB,
    pub light_grey: RGB,
    pub dark_grey: RGB,
    pub black: RGB,
}

impl DisplayColour {
    #[inline]
    pub fn get_color(&self, dmg_color: &DmgColor) -> RGB {
        match dmg_color {
            WHITE => self.white,
            LightGrey => self.light_grey,
            DarkGrey => self.dark_grey,
            BLACK => self.black,
        }
    }
}
