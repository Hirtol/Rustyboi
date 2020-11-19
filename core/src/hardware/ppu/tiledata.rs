use crate::hardware::ppu::palette::RGB;
use crate::hardware::ppu::register_flags::AttributeFlags;
use bitflags::_core::fmt::Formatter;
use std::fmt;
use std::fmt::Debug;

// 128 tiles each of 16 bytes each.
pub const TILE_BLOCK_0_START: u16 = 0x8000;
pub const TILE_BLOCK_0_END: u16 = 0x87FF;

pub const TILE_BLOCK_1_START: u16 = 0x8800;
pub const TILE_BLOCK_1_END: u16 = 0x8FFF;

pub const TILE_BLOCK_2_START: u16 = 0x9000;
pub const TILE_BLOCK_2_END: u16 = 0x97FF;

pub const BACKGROUND_TILE_SIZE: usize = 32 * 32;

pub const TILEMAP_9800_START: u16 = 0x9800;
pub const TILEMAP_9800_END: u16 = 0x9BFF;
pub const TILEMAP_9C00_START: u16 = 0x9C00;
pub const TILEMAP_9C00_END: u16 = 0x9FFF;

/// 8x8 pixels of 2 bits p.p => 16 bytes
/// Each Tile occupies 16 bytes, where each 2 bytes represent a line.
#[derive(Debug, Copy, Clone)]
pub struct Tile {
    pub data: [u8; 16],
    /// Contains the pixel colours taken from `data`
    pub unpaletted_pixels: [u8; 64],
}

/// Background Tile Map contains the numbers of tiles to be displayed.
/// It is organized as 32 rows of 32 bytes each. Each byte contains a number of a tile to be displayed.
///
/// Tile patterns are taken from the Tile Data Table using either of
/// the two addressing modes, which can be selected via LCDC register.
///
/// As one background tile has a size of 8x8 pixels,
/// the BG maps may hold a picture of 256x256 pixels,
/// and an area of 160x144 pixels of this picture can be displayed on the LCD screen.
pub struct TileMap {
    pub data: [u8; BACKGROUND_TILE_SIZE],
}

#[derive(Default, Copy, Clone)]
pub struct SpriteAttribute {
    /// Specifies the sprites vertical position on the screen (minus 16).
    /// An off-screen value (for example, Y=0 or Y>=160) hides the sprite.
    pub y_pos: u8,
    /// Specifies the sprites horizontal position on the screen (minus 8).
    /// An off-screen value (X=0 or X>=168) hides the sprite,
    /// but the sprite still affects the priority ordering -
    /// a better way to hide a sprite is to set its Y-coordinate off-screen.
    pub x_pos: u8,
    /// Specifies the sprites Tile Number (00-FF).
    /// This (unsigned) value selects a tile from memory at 8000h-8FFFh.
    /// In CGB Mode this could be either in VRAM Bank 0 or 1, depending on Bit 3 of the following byte.
    /// In 8x16 mode, the lower bit of the tile number is ignored.
    /// IE: the upper 8x8 tile is "NN AND FEh", and the lower 8x8 tile is "NN OR 01h".
    pub tile_number: u8,
    pub attribute_flags: AttributeFlags,
}

impl SpriteAttribute {
    /// Get a byte in the range `0..=3` from this sprite attribute.
    pub fn get_byte(&self, byte_num: u8) -> u8 {
        match byte_num {
            0 => self.y_pos,
            1 => self.x_pos,
            2 => self.tile_number,
            3 => self.attribute_flags.bits(),
            _ => panic!("Out of range byte number specified!"),
        }
    }

    /// Set a byte in the `byte_num` range `0..=3` to the specified `value`
    pub fn set_byte(&mut self, byte_num: u8, value: u8) {
        match byte_num {
            0 => self.y_pos = value,
            1 => self.x_pos = value,
            2 => self.tile_number = value,
            3 => self.attribute_flags = AttributeFlags::from_bits_truncate(value),
            _ => panic!("Out of range byte number specified!"),
        }
    }
}

impl Debug for SpriteAttribute {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sprite [ y_pos: {} - x_pos: {} - tile_number: 0x{:02X} - flags: {:?} ]",
            self.y_pos, self.x_pos, self.tile_number, self.attribute_flags
        )
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            data: [0; 16],
            unpaletted_pixels: [0; 64],
        }
    }
}

impl Tile {
    /// Get a single pre-calculated pixel from this tile.
    #[inline(always)]
    pub fn get_pixel(&self, index: usize) -> u8 {
        self.unpaletted_pixels[index]
    }

    /// Return an entire pre-computed pixel line, for most instances `get_pixel` is more
    /// efficient however.
    #[inline(always)]
    pub fn get_true_pixel_line(&self, start_index: usize) -> &[u8] {
        &self.unpaletted_pixels[start_index..start_index + 8]
    }

    /// Update the tile's data representation as well as its pre-computed palette colour cache.
    /// The current way this is done means that we'll do twice as many calculations as should strictly
    /// be required (since 2 bytes make up one pixel row, and we update after each byte is written).
    pub fn update_pixel_data(&mut self, byte_address: usize, value: u8) {
        let pixel_array_address = (byte_address - (byte_address % 2)) * 4;

        self.data[byte_address] = value;
        // If we're the even byte we need +1 for the pixel line, otherwise -1
        if (byte_address % 2) == 0 {
            let bottom_pixel_data = self.data[byte_address + 1];
            self.bulk_set_pixels(value, bottom_pixel_data, pixel_array_address);
        } else {
            let top_pixel_data = self.data[byte_address - 1];
            self.bulk_set_pixels(top_pixel_data, value, pixel_array_address);
        };
    }

    fn bulk_set_pixels(&mut self, top_pixel_data: u8, bottom_pixel_data: u8, pixel_array_address: usize) {
        self.unpaletted_pixels[pixel_array_address] = top_pixel_data & 1 | ((bottom_pixel_data & 1) << 1);
        self.unpaletted_pixels[pixel_array_address + 1] = (top_pixel_data & 2) >> 1 | (bottom_pixel_data & 2);
        self.unpaletted_pixels[pixel_array_address + 2] = (top_pixel_data & 4) >> 2 | ((bottom_pixel_data & 4) >> 1);
        self.unpaletted_pixels[pixel_array_address + 3] = (top_pixel_data & 8) >> 3 | ((bottom_pixel_data & 8) >> 2);
        self.unpaletted_pixels[pixel_array_address + 4] = (top_pixel_data & 16) >> 4 | ((bottom_pixel_data & 16) >> 3);
        self.unpaletted_pixels[pixel_array_address + 5] = (top_pixel_data & 32) >> 5 | ((bottom_pixel_data & 32) >> 4);
        self.unpaletted_pixels[pixel_array_address + 6] = (top_pixel_data & 64) >> 6 | ((bottom_pixel_data & 64) >> 5);
        self.unpaletted_pixels[pixel_array_address + 7] = (top_pixel_data & 128) >> 7 | ((bottom_pixel_data & 128) >> 6);
    }
}

impl TileMap {
    pub fn new() -> Self {
        TileMap {
            data: [0; BACKGROUND_TILE_SIZE],
        }
    }
}

impl Debug for TileMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.data[..].fmt(f)
    }
}
