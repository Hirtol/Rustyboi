use crate::io::palette::DmgColor::{WHITE, LightGrey, DarkGrey, BLACK};

#[derive(Debug, PartialOrd, PartialEq)]
pub enum DmgColor {
    WHITE     = 0x0,
    LightGrey = 0x1,
    DarkGrey  = 0x2,
    BLACK     = 0x3,
    // Doesn't really have a direct value. It's instead used by default for bit 0-1 in Sprites.
    TRANSPARENT,
}

#[derive(Debug)]
pub struct Palette {
    palette_byte: u8,
}

impl Palette {
    /// Return the color designation located at bit 0-1
    /// In Object (Sprite) palettes this particular color should be ignored, as it will always
    /// be transparent.
    pub fn color_0(&self) -> DmgColor {
        DmgColor::from(self.palette_byte & 0x03)
    }

    pub fn color_1(&self) -> DmgColor {
        DmgColor::from((self.palette_byte & 0x0C) >> 2)
    }

    pub fn color_2(&self) -> DmgColor {
        DmgColor::from((self.palette_byte & 0x30) >> 4)
    }

    pub fn color_3(&self) -> DmgColor {
        DmgColor::from((self.palette_byte & 0xC0) >> 6)
    }
}

impl From<u8> for Palette {
    fn from(value: u8) -> Self {
        Palette{palette_byte: value}
    }
}

impl From<u8> for DmgColor {
    fn from(value: u8) -> Self {
        match value {
            0x0 => WHITE,
            0x1 => LightGrey,
            0x2 => DarkGrey,
            0x3 => BLACK,
            _ => panic!("From u8 for DMGCOLOR should not reach this value! {}", value)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::io::palette::Palette;
    use crate::io::palette::DmgColor::{BLACK, LightGrey, WHITE, DarkGrey};

    #[test]
    fn test_palette_interpretation() {
        let palette = Palette::from(0b11010010);
        assert_eq!(palette.color_0(), DarkGrey);
        assert_eq!(palette.color_1(), WHITE);
        assert_eq!(palette.color_2(), LightGrey);
        assert_eq!(palette.color_3(), BLACK);
    }

}