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

    pub fn get_lcd_control(&self) -> u8 {
        self.lcd_control.bits()
    }

    pub fn get_lcd_status(&self) -> u8 {
        self.lcd_status.bits()
    }

    pub fn get_scy(&self) -> u8 {
        self.scroll_y
    }

    pub fn get_scx(&self) -> u8 {
        self.scroll_x
    }

    pub fn get_ly(&self) -> u8 {
        self.current_y
    }

    pub fn get_lyc(&self) -> u8 {
        self.compare_line
    }

    pub fn get_bg_palette(&self) -> u8 {
        self.bg_window_palette.into()
    }

    pub fn get_oam_palette_0(&self) -> u8 {
        self.oam_palette_0.into()
    }

    pub fn get_oam_palette_1(&self) -> u8 {
        self.oam_palette_1.into()
    }

    pub fn get_window_y(&self) -> u8 {
        self.window_y
    }

    pub fn get_window_x(&self) -> u8 {
        self.window_x
    }

    pub fn set_lcd_control(&mut self, value: u8) {
        self.lcd_control = LcdControl::from_bits_truncate(value);
    }

    pub fn set_lcd_status(&mut self, value: u8) {
        self.lcd_status = LcdStatus::from_bits_truncate(value);
    }

    pub fn set_scy(&mut self, value: u8) {
        self.scroll_y = value
    }

    pub fn set_scx(&mut self, value: u8) {
        self.scroll_x = value
    }

    pub fn set_ly(&mut self, value: u8) {
        self.current_y = value
    }

    pub fn set_lyc(&mut self, value: u8) {
        self.compare_line = value
    }

    pub fn set_bg_palette(&mut self, value: u8) {
        self.bg_window_palette = Palette::from(value)
    }

    pub fn set_oam_palette_0(&mut self, value: u8) {
        self.oam_palette_0 = Palette::from(value)
    }

    pub fn set_oam_palette_1(&mut self, value: u8) {
        self.oam_palette_1 = Palette::from(value)
    }

    pub fn set_window_y(&mut self, value: u8) {
        self.window_y = value
    }

    pub fn set_window_x(&mut self, value: u8) {
        self.window_x = value
    }



}

/// Get the internal PPU address for a tile from a normal u16 address.
/// Returns in the format `(tile_addresss, byte_address)`
fn get_tile_address(address: u16) -> (usize, usize){
    let relative_address = address as usize - TILE_BLOCK_0_START as usize;
    (relative_address / 16, relative_address % 16)
}
