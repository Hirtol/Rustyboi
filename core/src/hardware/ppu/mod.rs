use crate::emulator::MMU;
use crate::hardware::memory::{Memory, MemoryMapper};
use crate::hardware::ppu::palette::{Palette, DmgColor, DisplayColour, RGB};
use crate::hardware::ppu::register_flags::{LcdControl, LcdStatus};
use std::collections::VecDeque;
use crate::hardware::ppu::tiledata::{TILEMAP_9800, TILEMAP_9C00, TILE_BLOCK_0_START, TILE_BLOCK_1_START};

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

pub mod tiledata;
pub mod palette;
pub mod register_flags;

//TODO: Implement 10 sprite limit.
//TODO: Implement sprite priority (x-based, in case of tie, then first sprite in mem; start: 0xFE00)

pub struct PPU {
    frame_buffer: [u8; FRAMEBUFFER_SIZE],
    mmu: MMU<Memory>,
    pub colorisor: DisplayColour,
    //fifo_bg: FIFO,
    //Sprite FIFO.
    //fifo_oam: FIFO,
    current_x: u8,
    current_y: u8,
    last_call_cycles: u128,
    current_cycles: u128,
}

pub struct FIFO {
    queue: VecDeque<FIFOPixel>,
}

struct FIFOPixel {
    //color: u8,
    //palette: &'a Palette,
    render_color: DmgColor,
    /// Not necessarily necessary. Only if we want to implement x conflict maybe?
    //priority: u8,
    background_priority: bool,
}

impl PPU {
    pub fn new(mmu: &MMU<Memory>, display_colors: DisplayColour) -> Self {
        PPU { frame_buffer: [150; FRAMEBUFFER_SIZE], mmu: mmu.clone(), colorisor: display_colors, current_x: 0, current_y: 0, last_call_cycles: 0, current_cycles: 0 }
    }

    pub fn cycle(&mut self) {
        use crate::hardware::ppu::tiledata::*;
        let mmu = self.mmu.borrow_mut();

        let lcd_control: LcdControl = LcdControl::from_bits_truncate(mmu.read_byte(LCD_CONTROL_REGISTER));
        let lcd_status: LcdStatus = LcdStatus::from_bits_truncate(mmu.read_byte(LCD_STATUS_REGISTER));

        let scx = mmu.read_byte(SCX_REGISTER);
        let scy = mmu.read_byte(SCY_REGISTER);
        let window_x = mmu.read_byte(WX_REGISTER); //- 7 ?;
        let window_y = mmu.read_byte(WY_REGISTER);


        let mut tile_map_used = TILEMAP_9800;

        if (lcd_control.contains(LcdControl::BG_TILE_MAP_SELECT) && scx < window_x)
            || (lcd_control.contains(LcdControl::WINDOW_TILE_SELECT) && scx >= window_x){
            tile_map_used = TILEMAP_9C00;
        }

        let fetcher_x = ((scx / 8));


    }

    pub fn true_cycle(&mut self) {
        self.draw();
        let mut mmu = self.mmu.borrow_mut();
        let delta_cycles = mmu.cycles_performed() - self.last_call_cycles;
        self.current_cycles = (self.current_cycles + 1) % 456;

        if self.current_cycles == 0 {
            self.current_y = (self.current_y + 1) % 154;
            mmu.write_byte(LY_REGISTER, self.current_y);
        }
    }

    fn draw(&mut self) {


        let x = (self.current_x as usize + self.mmu.borrow().read_byte(SCX_REGISTER) as usize) % RESOLUTION_WIDTH;
        let y = (self.current_y as usize+ self.mmu.borrow().read_byte(SCY_REGISTER) as usize) % RESOLUTION_HEIGHT;
        self.set_rgb(self.colorisor.white, x as u8, y as u8);
        let mut tile_map_used = TILEMAP_9800;

        if (self.get_lcdc_register().contains(LcdControl::BG_TILE_MAP_SELECT) && self.mmu.borrow().read_byte(SCX_REGISTER) < self.mmu.borrow().read_byte(WX_REGISTER))
            || (self.get_lcdc_register().contains(LcdControl::WINDOW_TILE_SELECT) && self.mmu.borrow().read_byte(SCX_REGISTER) >= self.mmu.borrow().read_byte(WX_REGISTER)){
            tile_map_used = TILEMAP_9C00;
        }

        if self.get_lcdc_register().contains(LcdControl::BG_WINDOW_PRIORITY) {
            //TODO: Change to proper window data range.
            self.draw_bg_window(x as u8,y as u8, tile_map_used, if self.get_lcdc_register().contains(LcdControl::BG_WINDOW_TILE_SELECT) { TILE_BLOCK_0_START} else { TILE_BLOCK_1_START})
        }
    }

    fn draw_sprites(&self) {
        if !self.get_lcdc_register().contains(LcdControl::SPRITE_DISPLAY_ENABLE) {
            return;
        }


    }

    fn draw_bg_window(&mut self, x: u8, y: u8, tilemap_offset: u16, bg_window_data_range: u16) {
        let address = self.get_tile_data_address(x,y,tilemap_offset,bg_window_data_range);
        let tile_data = self.get_tile_pixel_data(address, x, y);

        let color = self.colorisor.get_color(self.get_bg_window_palette().color(tile_data));
        self.set_rgb(color, x, y);
    }

    /// Returns the BG/window tilemap ID to be used in the tile blocks (0x8000..=0x9FFF)
    /// for the current x and y pixel.
    fn get_tile_map_id(&self, x: u8, y: u8, tilemap_offset: u16) -> u8 {
        let tile_x = (x/8) as u16;
        let tile_y = (y/8) as u16;

        let tile_id_address = tilemap_offset + tile_x + (tile_y * 32);

        if tile_id_address > (tilemap_offset + 0x3FF) {
            panic!("Game tried to reach outside of bounds: {}", tile_id_address);
        }

        self.mmu.borrow().read_byte(tile_id_address)
    }

    /// Get the memory address for the tile data at x,y
    fn get_tile_data_address(&self, x: u8, y: u8, tilemap_offset: u16, bg_window_data_range: u16) -> u16 {
        let tile_id = self.get_tile_map_id(x,y, tilemap_offset);
        let mut address = bg_window_data_range;

        if self.get_lcdc_register().contains(LcdControl::BG_WINDOW_TILE_SELECT) {
            address = address.wrapping_add(tile_id as u16 * 16);
        } else {
            address = address.wrapping_add((tile_id as i8 * 16) as u16);
        }

        address
    }

    /// Returns the tile data for pixel x,y of tile at address addr
    pub fn get_tile_pixel_data(&self, address: u16, x: u8, y: u8) -> u8 {
        let dx = 7 - (x % 8);
        let dy = 2 * (y % 8) as u16;
        // Pixel data is spread over 2 bytes
        let a = self.mmu.borrow().read_byte(address + dy);
        let b = self.mmu.borrow().read_byte(address + dy + 1);

        let bit1 = (a & (1 << dx)) >> dx;
        let bit2 = (b & (1 << dx)) >> dx;

        bit1 | (bit2 << 1)
    }

    fn get_lcdc_register(&self) -> LcdControl {
        LcdControl::from_bits_truncate(self.mmu.borrow().read_byte(LCD_CONTROL_REGISTER))
    }

    pub fn get_bg_window_palette(&self) -> Palette {
        Palette::from(self.mmu.borrow().read_byte(BG_PALETTE))
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