use crate::hardware::ppu::palette::RGB;
use crate::hardware::ppu::tiledata::BACKGROUND_TILE_SIZE;
use bitflags::*;
use std::ops::Index;

#[derive(Debug)]
pub struct CgbTileMap {
    pub attributes: [CgbTileAttribute; BACKGROUND_TILE_SIZE],
}

impl CgbTileMap {
    pub fn new() -> Self {
        CgbTileMap {
            attributes: [CgbTileAttribute::default(); BACKGROUND_TILE_SIZE],
        }
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
    pub fn bg_palette_numb(&self) -> usize {
        (self.bits & 0x7) as usize
    }

    pub fn set_bg_palette_numb(&mut self, value: u8) {
        self.bits = (self.bits & 0xF8) | (value & 0x7);
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct CgbPaletteIndex {
    pub selected_address: usize,
    pub auto_increment: bool,
}

impl CgbPaletteIndex {
    pub fn set_value(&mut self, value: u8) {
        self.selected_address = (value as usize) & 0x3F;
        self.auto_increment = (value & 0x80) != 0;
    }

    pub fn get_value(&self) -> u8 {
        (self.selected_address as u8) | if self.auto_increment { 0x80 } else { 0x0 }
    }
}

#[derive(Default, Debug, Copy, Clone)]
pub struct CgbPalette {
    pub colours: [CgbRGBColour; 4],
}

impl CgbPalette {
    /// Retrieve the appropriate colour for the provided pixel value.
    ///
    /// Due to the aforementioned the `colour_value` should have at most 2 bits in use.
    #[inline(always)]
    pub fn colour(&self, color_value: u8) -> RGB {
        // We elide a panic!/unreachable! here since this will only ever be called internally,
        // and it gives a good speed boost.
        match color_value {
            0 => self.colours[0].rgb,
            1 => self.colours[1].rgb,
            2 => self.colours[2].rgb,
            _ => self.colours[3].rgb,
        }
    }

    pub fn rgb(&self) -> [RGB; 4] {
        [
            self.colours[0].rgb,
            self.colours[1].rgb,
            self.colours[2].rgb,
            self.colours[3].rgb,
        ]
    }
}

/// This struct will naively convert the written 15 bit colour values to 24 bit.
#[derive(Debug, Copy, Clone, Default)]
pub struct CgbRGBColour {
    pub rgb: RGB,
    r5: u8,
    g5: u8,
    b5: u8,
}

impl CgbRGBColour {
    pub fn set_high_byte(&mut self, value: u8) {
        self.b5 = (value & 0x7C) >> 2;
        self.g5 = (self.g5 & 0x07) | ((value & 0x03) << 3);
        // Formula taken from: https://stackoverflow.com/questions/2442576/how-does-one-convert-16-bit-rgb565-to-24-bit-rgb888
        self.rgb.2 = ((self.b5 as u32 * 527 + 23) >> 6) as u8;
        self.rgb.1 = ((self.g5 as u32 * 527 + 23) >> 6) as u8;
    }

    pub fn set_low_byte(&mut self, value: u8) {
        self.g5 = (self.g5 & 0x18) | ((value & 0xE0) >> 5);
        self.r5 = value & 0x1F;

        self.rgb.1 = ((self.g5 as u32 * 527 + 23) >> 6) as u8;
        self.rgb.0 = ((self.r5 as u32 * 527 + 23) >> 6) as u8;
    }

    pub fn get_high_byte(&self) -> u8 {
        (self.b5 << 2) | self.g5 & 0x3
    }

    pub fn get_low_byte(&self) -> u8 {
        (self.g5 << 5) | self.r5
    }
}

#[cfg(test)]
mod tests {
    use crate::hardware::ppu::cgb_vram::{CgbRGBColour, CgbTileAttribute};

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

    #[test]
    fn test_cgb_rgb() {
        let mut rgb = CgbRGBColour::default();
        rgb.set_high_byte(0xF8);
        rgb.set_low_byte(0x9F);
        assert_eq!(rgb.r5, 0b1_1111);
        assert_eq!(rgb.g5, 0b0_0100);
        assert_eq!(rgb.b5, 0b1_1110);
        assert_eq!(rgb.get_high_byte(), 0b0111_1000);
        assert_eq!(rgb.get_low_byte(), 0b1001_1111);

        let full_thing = 0b0_11001_00111_00111 as u16;
        rgb.set_high_byte(((full_thing & 0x7F00) >> 8) as u8);
        rgb.set_low_byte(full_thing as u8);

        assert_eq!(rgb.r5, 0b00111);
        assert_eq!(rgb.g5, 0b00111);
        assert_eq!(rgb.b5, 0b11001);
    }
}
