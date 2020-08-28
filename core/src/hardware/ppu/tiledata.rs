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

// 128 tiles each.
pub const TILE_BLOCK_0_START: u16 = 0x8000;
pub const TILE_BLOCK_0_END: u16 = 0x87FF;

pub const TILE_BLOCK_1_START: u16 = 0x8800;
pub const TILE_BLOCK_1_END: u16 = 0x8FFF;

pub const TILE_BLOCK_2_START: u16 = 0x9000;
pub const TILE_BLOCK_2_END: u16 = 0x97FF;

/// 8x8 pixels of 2 bits p.p => 16 bytes
/// Each Tile occupies 16 bytes, where each 2 bytes represent a line.
pub struct Tile {
    data: [u8; 16]
}

pub struct TileData {
    tiles: [Tile; 384]
}