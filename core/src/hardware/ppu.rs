
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

pub mod tiledata;
pub mod palette;

pub struct PPU {
    pub frame_buffer: [u8; FRAMEBUFFER_SIZE],

}

impl PPU {
    pub fn new(frame_buffer: [u8; 69120]) -> Self {
        PPU { frame_buffer }
    }
}
