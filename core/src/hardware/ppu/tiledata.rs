//! Tiles are always indexed using an 8-bit integer, but the addressing method may differ.
//! The "8000 method" uses $8000 as its base pointer and uses an unsigned addressing,
//! meaning that tiles 0-127 are in block 0, and tiles 128-255 are in block 1.
//! The "8800 method" uses $9000 as its base pointer and uses a signed addressing.
//! To put it differently, "8000 addressing" takes tiles 0-127 from block 0
//! and tiles 128-255 from block 1, whereas "8800 addressing" takes tiles 0-127
//! from block 2 and tiles 128-255 from block 1.
//! (You can notice that block 1 is shared by both addressing methods)
//!
//! Sprites always use 8000 addressing,
//! but the BG and Window can use either mode, controlled by LCDC bit 4 (`BG_WINDOW_TILE_SELECT`).

use crate::hardware::ppu::register_flags::AttributeFlags;
use bitflags::_core::fmt::Formatter;
use std::fmt;
use std::fmt::Debug;
use std::mem::size_of;
use std::ops::Index;

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
#[derive(Default, Debug, Copy, Clone)]
pub struct Tile {
    pub data: [u8; 16],
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

impl Tile {
    pub fn get_pixel_line(&self, mut line_y: u8) -> (u8, u8) {
        let address = (line_y * 2) as usize;
        (self.data[address], self.data[address + 1])
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
