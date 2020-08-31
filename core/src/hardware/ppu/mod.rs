use std::collections::VecDeque;

use crate::emulator::MMU;
use crate::hardware::memory::{Memory, MemoryMapper, INTERRUPTS_FLAG};
use crate::hardware::ppu::palette::{DisplayColour, DmgColor, Palette, RGB};
use crate::hardware::ppu::register_flags::{LcdControl, LcdStatus};
use crate::hardware::ppu::tiledata::*;
use crate::io::interrupts::InterruptFlags;

/// The DMG in fact has a 256x256 drawing area, whereupon a viewport of 160x144 is placed.
pub const TRUE_RESOLUTION_WIDTH: usize = 256;
pub const TRUE_RESOLUTION_HEIGHT: usize = 256;

pub const RESOLUTION_WIDTH: usize = 160;
pub const RESOLUTION_HEIGHT: usize = 144;
pub const RGB_CHANNELS: usize = 3;
pub const FRAMEBUFFER_SIZE: usize = RESOLUTION_HEIGHT * RESOLUTION_WIDTH * RGB_CHANNELS;

pub const LCD_CONTROL_REGISTER: u16 = 0xFF40;
pub const LCD_STATUS_REGISTER: u16 = 0xFF41;
/// Specifies the position in the 256x256 pixels BG map (32x32 tiles)
/// which is to be displayed at the upper/left LCD display position.
/// Values in range from 0-255 may be used for X/Y each,
/// the video controller automatically wraps back to the upper (left)
/// position in BG map when drawing exceeds the lower (right) border of the BG map area.
pub const SCY_REGISTER: u16 = 0xFF42;
pub const SCX_REGISTER: u16 = 0xFF43;
/// LCDC Y-Coordinate (R)
/// The LY indicates the vertical line to which the present data is transferred to the LCD Driver.
/// The LY can take on any value between 0 through 153.
/// The values between 144 and 153 indicate the V-Blank period.
pub const LY_REGISTER: u16 = 0xFF44;
/// LYC - LY Compare (R/W)
/// The Game Boy permanently compares the value of the LYC and LY registers.
/// When both values are identical, the coincident bit in the STAT register becomes set,
/// and (if enabled) a STAT interrupt is requested.
pub const LYC_REGISTER: u16 = 0xFF45;
/// Window Y Position (R/W)
///
/// Specifies the upper/left positions of the Window area.
/// (The window is an alternate background area which can be displayed above of the normal background.
/// Sprites may be still displayed above or behind the window, just as for normal BG.)
///
/// The window becomes visible (if enabled) when positions are set in range WX=0..166, WY=0..143.
/// A position of WX=7, WY=0 locates the window at upper left,
/// it is then completely covering normal background.
pub const WY_REGISTER: u16 = 0xFF4A;
/// Window X Position minus 7 (R/W)
pub const WX_REGISTER: u16 = 0xFF4B;
/// BG Palette Data (R/W) - Non CGB Mode Only
/// This register assigns gray shades to the color numbers of the BG and Window tiles.
/// In CGB Mode the Color Palettes are taken from CGB Palette Memory instead.
pub const BG_PALETTE: u16 = 0xFF47;
/// Object Palette 0 Data (R/W) - Non CGB Mode Only.
/// This register assigns gray shades for sprite palette 0.
/// It works exactly as BGP (FF47), except that the lower
/// two bits aren't used because sprite data 00 is transparent.
pub const OB_PALETTE_0: u16 = 0xFF48;
/// Object Palette 1 Data (R/W) - Non CGB Mode Only.
///
/// Same as [OB_PALETTE_0](const.OB_PALETTE_0.html)
pub const OB_PALETTE_1: u16 = 0xFF49;
//TODO: Add CGB color palettes.

/// DMA Transfer and Start Address (R/W).
/// Writing to this register launches a DMA transfer from ROM or RAM to OAM memory (sprite attribute table).
/// The written value specifies the transfer source address divided by 100h, ie. source & destination are:
///
/// ```text
/// Source:      XX00-XX9F   ;XX in range from 00-F1h
/// Destination: FE00-FE9F
/// ```
/// The transfer takes 160 machine cycles, definitely read more [here]
///
/// [here]: https://gbdev.io/pandocs/#lcd-oam-dma-transfers
pub const DMA_TRANSFER: u16 = 0xFF46;

// The SCY and SCX registers can be used to scroll the background,
// allowing to select the origin of the visible 160x144 pixel area within the total 256x256 pixel background map.
// Background wraps around the screen (i.e. when part of it goes off the screen, it appears on the opposite side.)

pub mod palette;
pub mod register_flags;
pub mod tiledata;
pub mod memory_binds;

//TODO: Implement 10 sprite limit.
//TODO: Implement sprite priority (x-based, in case of tie, then first sprite in mem; start: 0xFE00)

enum Mode {
    H_BLANK,
    V_BLANK,
    OAM_SEARCH,
    LCD_TRANSFER,
}

// Notes:
// OAM x and y coordinates are ALWAYS within viewport. E.g we can ignore SCX and SCY for those.
//

// Misc:
// If the Window is enabled while drawing the screen (LY is between 0 and 143)
// then if it is disabled by changing the Bit 5 in LCDC, the Game Boy "remembers"
// what line it was last rendering from the Window.
// If the Window, once disabled, is again enabled before VBlank,
// it starts drawing the Window from the last line it "remembers".

pub struct PPU {
    frame_buffer: [u8; FRAMEBUFFER_SIZE],
    scanline_buffer: [DmgColor; RESOLUTION_WIDTH],
    pub colorisor: DisplayColour,
    // VRAM Data
    tiles: [Tile; 384],
    tile_map_9800: TileMap,
    tile_map_9c00: TileMap,
    oam: [SpriteAttribute; 40],

    lcd_control: LcdControl,
    lcd_status: LcdStatus,

    pub bg_window_palette: Palette,
    pub oam_palette_0: Palette,
    pub oam_palette_1: Palette,

    compare_line: u8,

    current_y: u8,

    scroll_x: u8,
    scroll_y: u8,

    window_x: u8,
    window_y: u8,

    last_call_cycles: u128,
    current_cycles: u128,
}

impl PPU {
    pub fn new(display_colors: DisplayColour) -> Self {
        PPU {
            frame_buffer: [150; FRAMEBUFFER_SIZE],
            scanline_buffer: [DmgColor::WHITE; RESOLUTION_WIDTH],
            colorisor: display_colors,
            tiles: [Tile::default(); 384],
            tile_map_9800: TileMap::new(),
            tile_map_9c00: TileMap::new(),
            oam: [SpriteAttribute::default(); 40],
            lcd_control: Default::default(),
            lcd_status: Default::default(),
            bg_window_palette: Palette::default(),
            oam_palette_0: Palette::default(),
            oam_palette_1: Palette::default(),
            compare_line: 0,
            current_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            window_x: 0,
            window_y: 0,
            last_call_cycles: 0,
            current_cycles: 0,
        }
    }

    pub fn do_cycle(&mut self) {
        //TODO: Timing ?
        self.draw_scanline();
    }

    // Note: We'll handle interrupts outside the GPU, probably.
    pub fn draw_scanline(&mut self) {
        if self.lcd_control.contains(LcdControl::BG_WINDOW_PRIORITY) {
            self.draw_bg_scanline();

            if self.lcd_control.contains(LcdControl::WINDOW_DISPLAY) {
                self.draw_window_scanline();
            }
        }

        if self.lcd_control.contains(LcdControl::SPRITE_DISPLAY_ENABLE){
            self.draw_sprite_scanline();
        }
        // TODO: Consider moving this to the consumer of the emulator instead of within
        // Not really the business of the PPU to set the RGB representation.
        let mut current_address: usize = (self.current_y as usize * 3 * RESOLUTION_WIDTH);

        for (i, colour) in self.scanline_buffer.iter().enumerate() {
            let colour = self.colorisor.get_color(colour);
            current_address += i;
            self.frame_buffer[current_address] = colour.0;
            self.frame_buffer[current_address + 1] = colour.1;
            self.frame_buffer[current_address + 2] = colour.2;
        }

        self.current_y += 1;
    }

    fn draw_bg_scanline(&mut self) {
        let scanline_to_be_rendered = self.current_y.wrapping_add(self.scroll_y);
        // scanline_to_be_rendered can be in range 0-255, where each tile is 8 in length.
        // As we'll want to use this variable to index on the TileMap (1 byte pointer to tile)
        // We need to first divide by 8, to then multiply by 32 for our 1d representation array.
        let tile_lower_bound = (scanline_to_be_rendered / 8) as u16 * 32;
        let tile_higher_bound = tile_lower_bound as u16 + 32;

        let tile_pixel_y = scanline_to_be_rendered % 8;
        let mut pixel_counter = 0usize;
        //TODO: Currently not resilient to end of line partial tile rendering.
        for i in tile_lower_bound..tile_higher_bound {

            let mut tile_relative_address = self.get_tile_address_bg(i);
            if !self.lcd_control.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
                tile_relative_address = (tile_relative_address as i8) as u8;
            }

            let offset = if self.lcd_control.bg_window_tile_address() == TILE_BLOCK_0_START {0u8} else {128u8};
            let tile_address: usize = offset.wrapping_add(tile_relative_address) as usize;

            let tile: Tile = self.tiles[tile_address];

            let (top_pixel_data, bottom_pixel_data) = tile.get_pixel_line(tile_pixel_y);

            for j in 7..=0 {
                // Probably need to check the logic here.
                let mut current_pixel = (top_pixel_data & (0x1 << j)) >> j-1;
                current_pixel |= (bottom_pixel_data & (0x1 << j)) >> j;

                self.scanline_buffer[pixel_counter] = self.bg_window_palette.color(current_pixel);
                pixel_counter += 1;
            }
        }
    }

    fn draw_window_scanline(&mut self) {

    }

    fn draw_sprite_scanline(&mut self) {

    }

    fn get_tile_address_bg(&self, address: u16) -> u8 {
        if self.lcd_control.contains(LcdControl::BG_TILE_MAP_SELECT) {
            self.tile_map_9800.data[address as usize]
        } else {
            self.tile_map_9c00.data[address as usize]
        }
    }

    fn set_rgb(&mut self, rgb: RGB, x: u8, y: u8) {
        let address = (y as usize * RESOLUTION_HEIGHT as usize) + x as usize;
        self.frame_buffer[address] = rgb.0;
        self.frame_buffer[address + 1] = rgb.1;
        self.frame_buffer[address + 2] = rgb.2;
    }

    pub fn frame_buffer(&self) -> &[u8; FRAMEBUFFER_SIZE] {
        &self.frame_buffer
    }
}

#[test]
fn bo() {
    let test: u8 = (-20i8) as u8;
    let trues = 20u8;
    println!("{}", trues.wrapping_add(test))
}
