use bitflags::*;
use crate::hardware::ppu::tiledata::BACKGROUND_TILE_SIZE;
use std::ops::Index;

#[derive(Debug)]
pub struct CgbTileMap {
    pub attributes: [CgbTileAttribute; BACKGROUND_TILE_SIZE],
}

impl CgbTileMap {
    pub fn new() -> Self {
        CgbTileMap { attributes: [CgbTileAttribute::default(); BACKGROUND_TILE_SIZE] }
    }
}

bitflags! {
    #[derive(Default)]
    pub struct CgbTileAttribute: u8 {
        ///BGP 0-7
        const BG_PALETTE_NUMBER = 0b0000_0111;
        ///0=Bank 0, 1=Bank 1
        const TILE_VRAM_BANK_NUMBER = 0b0000_1000;
        /// Purely so that the full byte is transferred.
        const UNUSED = 0b0001_0000;
        /// (0=Normal, 1=Horizontally mirrored)
        const X_FLIP = 0b0010_0000;
        /// (0=Normal, 1=Vertically mirrored)
        const Y_FLIP = 0b0100_0000;
        /// (0=Use OAM priority bit, 1=BG Priority)
        const BG_TO_OAM_PRIORITY = 0b1000_0000;
    }
}

impl CgbTileAttribute {
    /// Returns the BG palette number in the range `0..=7`
    pub fn bg_palette_numb(&self) -> u8 {
        self.bits & 0x7
    }

    pub fn set_bg_palette_numb(&mut self, value: u8) {
        self.bits = (self.bits & 0xF8) | (value & 0x7);
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct CgbBgPaletteIndex {
    pub selected_address: usize,
    pub auto_increment: bool,
}

impl CgbBgPaletteIndex {
    pub fn set_value(&mut self, value: u8) {
        self.selected_address = (value as usize) & 0x3F;
        self.auto_increment = (value & 0x80) == 1
    }

    pub fn get_value(&self) -> u8 {
        (self.selected_address as u8) | if self.auto_increment {0x80} else {0x0}
    }
}

#[derive(Default, Debug, Copy, Clone)]
/// This struct will naively convert the written 15 bit colour values to 24 bit.
pub struct CgbRGBColour{
    pub red: u8,
    pub green: u8,
    pub blue: u8
}

impl CgbRGBColour {
    pub fn set_high_byte(&mut self, value: u8) {
        self.blue = (value & 0x7C) >> 2;
        self.green = (self.green & 0x1C) | (value & 0x03);
    }

    pub fn set_low_byte(&mut self, value: u8) {
        self.green = (self.green & 0x03) | ((value & 0xE0) >> 3);
        self.red = value & 0x1F;
    }

    pub fn get_high_byte(&self) -> u8 {
        //TODO
        unimplemented!()
    }

    pub fn get_low_byte(&self) -> u8 {
        //TODO
        unimplemented!()
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct CgbPalette {
    pub colours: [CgbRGBColour; 4],
}

impl Index<usize> for CgbPalette {
    type Output = CgbRGBColour;

    fn index(&self, index: usize) -> &Self::Output {
        &self.colours[index]
    }
}


#[cfg(test)]
mod tests {
    use crate::hardware::ppu::cgb_vram::CgbTileAttribute;

    #[test]
    fn test_palette_numb() {
        let mut attr = CgbTileAttribute::default();

        assert_eq!(attr.bg_palette_numb(), 0);
        attr.set_bg_palette_numb(3);
        assert_eq!(attr.bg_palette_numb(), 3);
        attr.set_bg_palette_numb(7);
        assert_eq!(attr.bg_palette_numb(), 7);
        attr.set_bg_palette_numb(15);
        assert_eq!(attr.bg_palette_numb(), 7);
    }
}

