use crate::hardware::ppu::PPU;
use super::*;

impl PPU{

    pub fn get_tile_byte(&self, address: u16) -> u8 {
        let (tile_address, byte_address) = get_tile_address(address);

        self.tiles[tile_address].data[byte_address]
    }

    pub fn set_tile_byte(&mut self, address: u16, value: u8) {
        let (tile_address, byte_address) = get_tile_address(address);

        self.tiles[tile_address].data[byte_address] = value;
    }

    pub fn get_tilemap_byte(&self, address: u16) -> u8 {
        match address {
            TILEMAP_9800_START..=TILEMAP_9800_END => self.tile_map_9800.data[(address - TILEMAP_9800_START) as usize],
            // 9C00, assuming no malicious calls
            _ => self.tile_map_9c00.data[(address - TILEMAP_9C00_START) as usize],
        }
    }

    pub fn set_tilemap_byte(&mut self, address: u16, value: u8) {
        match address {
            TILEMAP_9800_START..=TILEMAP_9800_END => self.tile_map_9800.data[(address - TILEMAP_9800_START) as usize] = value,
            // 9C00, assuming no malicious calls
            _ => self.tile_map_9c00.data[(address - TILEMAP_9C00_START) as usize] = value,
        }
    }

}

/// Get the internal PPU address for a tile from a normal u16 address.
/// Returns in the format `(tile_addresss, byte_address)`
#[inline]
fn get_tile_address(address: u16) -> (usize, usize){
    let relative_address = address as usize - TILE_BLOCK_0_START as usize;
    (relative_address / 16, relative_address % 16)
}
