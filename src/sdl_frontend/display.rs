use rustyboi_core::hardware::ppu::palette::DmgColor::*;
use rustyboi_core::hardware::ppu::palette::DmgColor;

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