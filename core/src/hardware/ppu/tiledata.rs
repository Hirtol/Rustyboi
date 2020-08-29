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
use std::mem::size_of;

// 128 tiles each.
pub const TILE_BLOCK_0_START: u16 = 0x8000;
pub const TILE_BLOCK_0_END: u16 = 0x87FF;

pub const TILE_BLOCK_1_START: u16 = 0x8800;
pub const TILE_BLOCK_1_END: u16 = 0x8FFF;

pub const TILE_BLOCK_2_START: u16 = 0x9000;
pub const TILE_BLOCK_2_END: u16 = 0x97FF;

pub const BACKGROUND_TILE_SIZE: usize = 32*32;

pub const TILEMAP_9800: u16 = 0x9800;
pub const TILEMAP_9C00: u16 = 0x9C00;

/// 8x8 pixels of 2 bits p.p => 16 bytes
/// Each Tile occupies 16 bytes, where each 2 bytes represent a line.
pub struct Tile {
    data: [u8; 16]
}

/// Background Tile Map contains the numbers of tiles to be displayed.
/// It is organized as 32 rows of 32 bytes each. Each byte contains a number of a tile to be displayed.
///
/// Tile patterns are taken from the Tile Data Table using either of
/// the two addressing modes (described above), which can be selected via LCDC register.
///
/// As one background tile has a size of 8x8 pixels,
/// the BG maps may hold a picture of 256x256 pixels,
/// and an area of 160x144 pixels of this picture can be displayed on the LCD screen.
pub struct BackgroundTileMap {
    data: [u8; BACKGROUND_TILE_SIZE],
}

pub struct TileData {
    pub tiles: [Tile; 384],
    pub background_tile_0: BackgroundTileMap,
    pub background_tile_1: BackgroundTileMap,
}

pub struct SpriteAttribute {
    /// Specifies the sprites vertical position on the screen (minus 16).
    /// An off-screen value (for example, Y=0 or Y>=160) hides the sprite.
    y_pos: u8,
    /// Specifies the sprites horizontal position on the screen (minus 8).
    /// An off-screen value (X=0 or X>=168) hides the sprite,
    /// but the sprite still affects the priority ordering -
    /// a better way to hide a sprite is to set its Y-coordinate off-screen.
    x_pos: u8,
    /// Specifies the sprites Tile Number (00-FF).
    /// This (unsigned) value selects a tile from memory at 8000h-8FFFh.
    /// In CGB Mode this could be either in VRAM Bank 0 or 1, depending on Bit 3 of the following byte.
    /// In 8x16 mode, the lower bit of the tile number is ignored.
    /// IE: the upper 8x8 tile is "NN AND FEh", and the lower 8x8 tile is "NN OR 01h".
    tile_number: u8,
    attribute_flags: AttributeFlags,
}

