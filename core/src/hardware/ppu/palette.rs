#[derive(Debug, Copy, Clone)]
pub struct Palette {
    palette_byte: u8,
    // Index into the PPUs display colour array.
    pub colours: [RGB; 4],
}

#[derive(Copy, Clone, Debug, Default, PartialOrd, PartialEq)]
pub struct RGB(pub u8, pub u8, pub u8);

#[derive(Debug, Default, Copy, Clone)]
pub struct DisplayColour {
    pub white: RGB,
    pub light_grey: RGB,
    pub dark_grey: RGB,
    pub black: RGB,
}

impl DisplayColour {
    pub fn get_colour(&self, val: usize) -> RGB {
        match val {
            0 => self.white,
            1 => self.light_grey,
            2 => self.dark_grey,
            _ => self.black,
        }
    }
}

impl Palette {
    pub fn new(value: u8, display_colours: DisplayColour) -> Self {
        let value = value as usize;
        Palette {
            palette_byte: value as u8,
            colours: [
                display_colours.get_colour(value & 0x03),
                display_colours.get_colour((value & 0x0C) >> 2),
                display_colours.get_colour((value & 0x30) >> 4),
                display_colours.get_colour(value >> 6),
            ],
        }
    }
    /// Return the color designation located at bit 0-1
    /// In Object (Sprite) palettes this particular color should be ignored, as it will always
    /// be transparent.
    pub fn color_0(&self) -> RGB {
        self.colours[0]
    }

    pub fn color_1(&self) -> RGB {
        self.colours[1]
    }

    pub fn color_2(&self) -> RGB {
        self.colours[2]
    }

    pub fn color_3(&self) -> RGB {
        self.colours[3]
    }

    /// Retrieve the appropriate colour for the provided pixel value.
    ///
    /// Due to the aforementioned the `colour_value` should have at most 2 bits in use.
    pub fn colour(&self, color_value: u8) -> RGB {
        //TODO: Check if the performance benefit of omitting a panic stays, or is simply cache realignment
        // (at time of writing increases fps ~200)
        match color_value {
            0 => self.colours[0],
            1 => self.colours[1],
            2 => self.colours[2],
            _ => self.colours[3],
            //_ => panic!("This should not be reached, colour value: {}", color_value),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            palette_byte: 0b1110_0100,
            colours: [RGB::default(); 4],
        }
    }
}

impl Into<u8> for Palette {
    fn into(self) -> u8 {
        self.palette_byte
    }
}
