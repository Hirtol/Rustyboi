use bitflags::*;
use crate::hardware::ppu::tiledata::BACKGROUND_TILE_SIZE;

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

