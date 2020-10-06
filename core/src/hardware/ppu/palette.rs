use crate::hardware::ppu::palette::DmgColor::{DarkGrey, LightGrey, BLACK, WHITE};

#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum DmgColor {
    WHITE = 0x0,
    LightGrey = 0x1,
    DarkGrey = 0x2,
    BLACK = 0x3,
}

#[derive(Debug, Copy, Clone)]
pub struct Palette {
    palette_byte: u8,
    color_0: DmgColor,
    color_1: DmgColor,
    color_2: DmgColor,
    color_3: DmgColor,
}

impl Palette {
    /// Return the color designation located at bit 0-1
    /// In Object (Sprite) palettes this particular color should be ignored, as it will always
    /// be transparent.
    pub fn color_0(&self) -> DmgColor {
        self.color_0
    }

    pub fn color_1(&self) -> DmgColor {
        self.color_1
    }

    pub fn color_2(&self) -> DmgColor {
        self.color_2
    }

    pub fn color_3(&self) -> DmgColor {
        self.color_3
    }

    /// Retrieve the appropriate colour for the provided pixel value.
    ///
    /// Due to the aforementioned the `colour_value` should have at most 2 bits in use.
    pub fn colour(&self, color_value: u8) -> DmgColor {
        match color_value {
            0 => self.color_0,
            1 => self.color_1,
            2 => self.color_2,
            3 => self.color_3,
            _ => panic!("This should not be reached, color value: {}", color_value),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            palette_byte: 0b1110_0100,
            color_0: DmgColor::from(0x0),
            color_1: DmgColor::from(0x1),
            color_2: DmgColor::from(0x2),
            color_3: DmgColor::from(0x3),
        }
    }
}

impl From<u8> for Palette {
    fn from(value: u8) -> Self {
        Palette {
            palette_byte: value,
            color_0: DmgColor::from(value & 0x03),
            color_1: DmgColor::from((value & 0x0C) >> 2),
            color_2: DmgColor::from((value & 0x30) >> 4),
            color_3: DmgColor::from(value >> 6),
        }
    }
}

impl Into<u8> for Palette {
    fn into(self) -> u8 {
        self.palette_byte
    }
}

// Note, this should really be TryFrom, as currently we technically break the contract by including
// panic!
impl From<u8> for DmgColor {
    fn from(value: u8) -> Self {
        match value {
            0x0 => WHITE,
            0x1 => LightGrey,
            0x2 => DarkGrey,
            0x3 => BLACK,
            _ => panic!("From u8 for DMGCOLOR should not reach this value! {}", value),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::hardware::ppu::palette::DmgColor::{DarkGrey, LightGrey, BLACK, WHITE};
    use crate::hardware::ppu::palette::Palette;

    #[test]
    fn test_palette_interpretation() {
        let palette = Palette::from(0b11010010);
        assert_eq!(palette.color_0(), DarkGrey);
        assert_eq!(palette.color_1(), WHITE);
        assert_eq!(palette.color_2(), LightGrey);
        assert_eq!(palette.color_3(), BLACK);
    }
}
