use itertools::Itertools;
use num_integer::Integer;

use crate::emulator::{GameBoyModel, CYCLES_PER_FRAME};
use crate::hardware::ppu::cgb_vram::{CgbPalette, CgbPaletteIndex, CgbTileMap};
use crate::hardware::ppu::palette::{DisplayColour, Palette, RGB};
use crate::hardware::ppu::register_flags::*;
use crate::hardware::ppu::tiledata::*;
use crate::hardware::ppu::Mode::{HBlank, LcdTransfer, OamSearch, VBlank};
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::scheduler::{Event, EventType, Scheduler};

pub const RESOLUTION_WIDTH: usize = 160;
pub const RESOLUTION_HEIGHT: usize = 144;
pub const RGB_CHANNELS: usize = 3;
pub const FRAMEBUFFER_SIZE: usize = RESOLUTION_HEIGHT * RESOLUTION_WIDTH;

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
pub const CGB_VRAM_BANK_REGISTER: u16 = 0xFF4F;
/// DMA Transfer and Start Address (R/W).
/// Writing to this register launches a DMA transfer from ROM or RAM to OAM memory (sprite attribute table).
/// The written value specifies the transfer source address divided by 100h, ie. source & destination are:
///
/// ```text
/// Source:      XX00-XX9F   ;XX in range from 00-F1h
/// Destination: FE00-FE9F
/// ```
/// The transfer takes 160 machine cycles.
pub const DMA_TRANSFER: u16 = 0xFF46;
/// This register is used to address a byte in the CGBs Background Palette Memory.
/// Each two byte in that memory define a color value. The first 8 bytes define Color 0-3 of Palette 0 (BGP0), and so on for BGP1-7.
pub const CGB_BACKGROUND_COLOR_INDEX: u16 = 0xFF68;
/// his register allows to read/write data to the CGBs Background Palette Memory, addressed through Register FF68.
/// Each color is defined by two bytes (Bit 0-7 in first byte).
pub const CGB_BACKGROUND_PALETTE_DATA: u16 = 0xFF69;
/// These registers are used to initialize the Sprite Palettes OBP0-7
pub const CGB_SPRITE_COLOR_INDEX: u16 = 0xFF6A;
pub const CGB_OBJECT_PALETTE_DATA: u16 = 0xFF6B;
///This register serves as a flag for which object priority mode to use. While the DMG prioritizes
///objects by x-coordinate, the CGB prioritizes them by location in OAM.
/// This flag is set by the CGB bios after checking the game's CGB compatibility.
pub const CGB_OBJECT_PRIORITY_MODE: u16 = 0xFF6C;

pub mod cgb_ppu;
pub mod cgb_vram;
pub mod debugging_features;
pub mod dma;
pub mod memory_binds;
pub mod palette;
pub mod register_flags;
pub mod tiledata;

#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum Mode {
    HBlank = 0x0,
    VBlank = 0x1,
    OamSearch = 0x2,
    LcdTransfer = 0x3,
}

pub struct PPU {
    frame_buffer: [RGB; FRAMEBUFFER_SIZE],
    scanline_buffer: [RGB; RESOLUTION_WIDTH],
    // Bool is used for BG-to-OAM priority
    scanline_buffer_unpalette: [(u8, bool); RESOLUTION_WIDTH],
    // 768 tiles for CGB mode, 384 for DMG mode.
    tiles: [Tile; 768],
    tile_bank_currently_used: u8,
    tile_map_9800: TileMap,
    tile_map_9c00: TileMap,
    cgb_9800_tile_map: CgbTileMap,
    cgb_9c00_tile_map: CgbTileMap,
    pub oam: [SpriteAttribute; 40],

    lcd_control: LcdControl,
    lcd_status: LcdStatus,

    bg_window_palette: Palette,
    oam_palette_0: Palette,
    oam_palette_1: Palette,
    cgb_bg_palette_ind: CgbPaletteIndex,
    cgb_sprite_palette_ind: CgbPaletteIndex,
    cgb_bg_palette: [CgbPalette; 8],
    cgb_sprite_palette: [CgbPalette; 8],

    pub current_y: u8,
    lyc_compare: u8,

    scroll_x: u8,
    scroll_y: u8,

    window_x: u8,
    window_y: u8,
    window_counter: u8,
    window_triggered: bool,

    oam_transfer_ongoing: bool,
    /// (false=OAM Priority, true=Coordinate Priority)
    cgb_object_priority: bool,
    stat_irq_triggered: bool,
    /// Whether to use the CGB scanline renderer
    cgb_rendering: bool,
    emulated_model: GameBoyModel,
    /// Advanced timing and synchronisation.
    latest_access: u64,
    current_lcd_transfer_duration: u64,
}

impl PPU {
    /// Instantiates a PPU with the provided `DisplayColour`.
    /// The PPU will output a framebuffer with RGB24 values based on the `DisplayColour`
    /// if the emulator is running in `DMG` mode.
    ///
    /// Otherwise, the PPU will use CGB registers. In addition we proceed to load
    /// `DisplayColour` into the CGB palette registries (BG0, OBJ0, OBJ1) and we'll
    /// *always* use those three registries as our source of `RGB` colour (Both in `DMG` and `CGB`).
    pub fn new(
        bg_display_colour: DisplayColour,
        sp0_display: DisplayColour,
        sp1_display: DisplayColour,
        cgb_rendering: bool,
        gb_model: GameBoyModel,
    ) -> Self {
        let (cgb_bg_palette, cgb_sprite_palette) = initialise_cgb_palette(bg_display_colour, sp0_display, sp1_display);
        PPU {
            frame_buffer: [RGB::default(); FRAMEBUFFER_SIZE],
            scanline_buffer: [RGB::default(); RESOLUTION_WIDTH],
            scanline_buffer_unpalette: [(0, false); RESOLUTION_WIDTH],
            tiles: [Tile::default(); 768],
            tile_bank_currently_used: 0,
            tile_map_9800: TileMap::new(),
            tile_map_9c00: TileMap::new(),
            cgb_9800_tile_map: CgbTileMap::new(),
            cgb_9c00_tile_map: CgbTileMap::new(),
            oam: [SpriteAttribute::default(); 40],
            lcd_control: LcdControl::from_bits_truncate(0b1001_0011),
            lcd_status: LcdStatus::from_bits_truncate(0b1000_0001),
            bg_window_palette: Palette::new(0b1110_0100, DisplayColour::from(cgb_bg_palette[0].rgb())),
            oam_palette_0: Palette::new(0b1110_0100, DisplayColour::from(cgb_sprite_palette[0].rgb())),
            oam_palette_1: Palette::new(0b1110_0100, DisplayColour::from(cgb_sprite_palette[1].rgb())),
            cgb_bg_palette_ind: CgbPaletteIndex::default(),
            cgb_sprite_palette_ind: CgbPaletteIndex::default(),
            cgb_bg_palette,
            cgb_sprite_palette,
            lyc_compare: 0,
            current_y: 0,
            scroll_x: 0,
            scroll_y: 0,
            window_x: 0,
            window_y: 0,
            window_counter: 0,
            window_triggered: false,
            oam_transfer_ongoing: false,
            cgb_object_priority: true,
            stat_irq_triggered: false,
            cgb_rendering,
            emulated_model: gb_model,
            latest_access: 0,
            current_lcd_transfer_duration: 0
        }
    }

    pub fn oam_search(&mut self, interrupts: &mut Interrupts) {
        // After V-Blank we don't want to trigger the interrupt immediately.
        if self.lcd_status.mode_flag() != VBlank {
            self.current_y = self.current_y.wrapping_add(1);
            self.ly_lyc_compare(interrupts);
        }

        self.lcd_status.set_mode_flag(Mode::OamSearch);
        // OAM Interrupt
        self.request_stat_interrupt(interrupts);
    }

    #[inline]
    pub fn get_lcd_transfer_duration(&mut self) -> u64 {
        self.current_lcd_transfer_duration = self.calculate_lcd_transfer_duration();
        self.current_lcd_transfer_duration
    }

    #[inline]
    pub fn get_hblank_duration(&self) -> u64 {
        // Hblank lasts at most 204 cycles.
        376 - self.current_lcd_transfer_duration
    }

    #[inline]
    fn calculate_lcd_transfer_duration(&self) -> u64 {
        // All cycles mentioned here are t-cycles
        let mut base_cycles = 172;
        // If we need to skip a few initial pixels this scanline.
        base_cycles += (self.scroll_x % 8) as u64;

        // If there's an active window the fifo pauses for *at least* 6 cycles.
        // TODO: 'at least' implies there can be more?
        if self.window_triggered && self.window_x < 168 && self.lcd_control.contains(LcdControl::WINDOW_DISPLAY) {
            base_cycles += 6;
        }
        // TODO: Dedup this, as currently it's copied from draw_sprite_scanline()
        let tall_sprites = self.lcd_control.contains(LcdControl::SPRITE_SIZE);
        let y_size: u8 = if tall_sprites { 16 } else { 8 };
        // Every sprite will *usually* pause for `11 - min(5, (x + SCX) mod 8)` cycles.
        // If drawn over the window will use 255 - WX instead of SCX.
        base_cycles += self.oam.iter()
            .filter(|sprite| {
                let screen_y_pos = sprite.y_pos as i16 - 16;
                is_sprite_on_scanline(self.current_y as i16, screen_y_pos, y_size as i16)
            })
            .take(10) // Max 10 sprites per scanline
            .map(|s| {
                let to_add = if self.window_triggered && self.window_x >= s.x_pos {
                    255 - self.window_x
                } else {
                    self.scroll_x
                };

                11 - core::cmp::min(5, (s.x_pos + to_add) % 8)
            })
            .sum::<u8>() as u64;

        //log::warn!("Calculated lcd duration: {} t-cycles", base_cycles);
        base_cycles
    }

    pub fn lcd_transfer(&mut self) {
        // Drawing (Mode 3)
        self.lcd_status.set_mode_flag(LcdTransfer);

        // Draw our actual line once we enter Drawing mode.
        if self.cgb_rendering {
            self.draw_cgb_scanline();
        } else {
            self.draw_scanline();
        }
    }

    pub fn hblank(&mut self, interrupts: &mut Interrupts) {
        self.lcd_status.set_mode_flag(HBlank);

        self.request_stat_interrupt(interrupts);
    }

    pub fn vblank(&mut self, interrupts: &mut Interrupts) {
        self.lcd_status.set_mode_flag(VBlank);

        // Check for line 144 lyc.
        self.current_y = self.current_y.wrapping_add(1);
        self.ly_lyc_compare(interrupts);

        self.window_counter = 0;
        self.window_triggered = false;
        // Check for Vblank flag in LCD Stat
        self.request_stat_interrupt(interrupts);

        interrupts.insert_interrupt(InterruptFlags::VBLANK);
    }

    pub fn vblank_wait(&mut self, interrupts: &mut Interrupts) {
        self.current_y = self.current_y.wrapping_add(1);
        self.ly_lyc_compare(interrupts);
    }

    /// On line 153 (the line right before we transfer to OAM Search) the LY register only
    /// reads 153 for 4 cycles, this method will set LY to 0 and do the ly_lyc compare.
    pub fn late_y_153_to_0(&mut self, interrupts: &mut Interrupts) {
        self.current_y = 0;
        self.ly_lyc_compare(interrupts);
    }

    pub fn draw_scanline(&mut self) {
        // As soon as wy == ly ANYWHERE in the frame, the window will be considered
        // triggered for the remainder of the frame, and thus can only be disabled
        // if LCD Control WINDOW_DISPlAY is reset.
        // This trigger can happen even if the WINDOW_DISPLAY bit is not set.
        if !self.window_triggered {
            self.window_triggered = self.current_y == self.window_y;
        }

        if self.lcd_control.contains(LcdControl::BG_WINDOW_PRIORITY) {
            if self.lcd_control.contains(LcdControl::WINDOW_DISPLAY) {
                if !self.window_triggered || self.window_x > 7 {
                    self.draw_bg_scanline();
                }
                self.draw_window_scanline();
            } else {
                self.draw_bg_scanline()
            }
        } else {
            let bg_colour = self.bg_window_palette.color_0();
            for pixel in self.scanline_buffer.iter_mut() {
                *pixel = bg_colour;
            }
        }

        if self.lcd_control.contains(LcdControl::SPRITE_DISPLAY_ENABLE) {
            self.draw_sprite_scanline();
        }

        let current_address: usize = self.current_y as usize * RESOLUTION_WIDTH;

        // Copy the value of the current scanline to the framebuffer.
        self.frame_buffer[current_address..current_address + RESOLUTION_WIDTH].copy_from_slice(&self.scanline_buffer);
    }

    fn draw_bg_scanline(&mut self) {
        let scanline_to_be_rendered = self.current_y.wrapping_add(self.scroll_y);
        // scanline_to_be_rendered can be in range 0-255, where each tile is 8 in length.
        // As we'll want to use this variable to index on the TileMap (1 byte pointer to tile)
        // We need to first divide by 8, to then multiply by 32 for our array (32*32) with a 1d representation.
        let tile_lower_bound = ((scanline_to_be_rendered / 8) as u16 * 32) + (self.scroll_x / 8) as u16;
        // 20 since 20*8 = 160 pixels
        let mut tile_higher_bound = (tile_lower_bound + 20);

        // Which particular y coordinate to use from an 8x8 tile.
        let tile_pixel_y = (scanline_to_be_rendered as usize % 8) * 8;
        let tile_pixel_y_offset = (tile_pixel_y + 7);
        // How many pixels we've drawn so far on this scanline.
        let mut pixel_counter: i16 = 0;
        // The amount of pixels to skip from the first tile in the sequence, and partially render
        // the remainder of that tile.
        // (for cases where self.scroll_x % 8 != 0, and thus not nicely aligned on tile boundaries)
        let mut pixels_to_skip = self.scroll_x % 8;
        // If the tile is not nicely aligned on % 8 boundaries we'll need an additional tile for the
        // last 8-pixels_to_skip pixels of the scanline.
        if pixels_to_skip != 0 {
            tile_higher_bound += 1;
        }

        for mut i in tile_lower_bound..tile_higher_bound {
            // When we wraparound in the x direction we want to stay on the same internal y-tile
            // Since we have a 1d representation of the tile map we have to subtract 32 to 'negate'
            // the effect of the x wraparound (since this wraparound
            // would have us go to the next y-tile line in the tile map)
            if (self.scroll_x as u16 + pixel_counter as u16) > 255 {
                i -= 32;
            }
            // Modulo for the y-wraparound if scroll_y > 111
            let mut tile_relative_address = self.get_tile_address_bg(i % BACKGROUND_TILE_SIZE as u16) as usize;
            let mut tile_address = tile_relative_address;

            // If we've selected the 8800-97FF mode we need to add a 256 offset, and then
            // add/subtract the relative address. (since we can then reach tiles 128-384)
            if !self.lcd_control.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
                tile_address = (256_usize).wrapping_add((tile_relative_address as i8) as usize);
            }

            self.draw_background_window_line(
                &mut pixel_counter,
                &mut pixels_to_skip,
                tile_address,
                tile_pixel_y,
                tile_pixel_y_offset,
            )
        }
    }

    fn draw_window_scanline(&mut self) {
        // -7 is apparently necessary for some reason
        // We need the i16 cast as there are games (like Aladdin) which have a wx < 7, but still
        // want their windows to be rendered.
        let mut window_x = (self.window_x as i16).wrapping_sub(7);
        // If the window x is out of scope, don't bother rendering.
        if !self.window_triggered || window_x >= 160 {
            return;
        }
        // The window always start to pick tiles from the top left of its BG tile map,
        // and has a separate line counter for its
        let tile_lower_bound = ((self.window_counter / 8) as u16) * 32;
        // We need as many tiles as there are to the end of the current scanline, even if they're
        // partial, therefore we need a ceiling divide.
        let tile_higher_bound = (tile_lower_bound as u16 + ((160 - window_x) as u16).div_ceil(&8)) as u16;

        let tile_pixel_y = (self.window_counter as usize % 8) * 8;
        let tile_pixel_y_offset = (tile_pixel_y + 7);

        // If window is less than 0 we want to skip those amount of pixels, otherwise we render as normal.
        // This means that we must take the absolute value of window_x for the pixels_skip, therefore the -
        let (mut pixel_counter, mut pixels_to_skip) = if window_x >= 0 {
            (window_x, 0)
        } else {
            (0, (-window_x) as u8)
        };

        self.window_counter += 1;

        for i in tile_lower_bound..tile_higher_bound {
            let mut tile_relative_address = self.get_tile_address_window(i) as usize;
            let mut tile_address = tile_relative_address;

            // If we've selected the 8800-97FF mode we need to add a 256 offset, and then
            // add/subtract the relative address.
            if !self.lcd_control.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
                tile_address = (256_usize).wrapping_add((tile_relative_address as i8) as usize);
            }

            self.draw_background_window_line(
                &mut pixel_counter,
                &mut pixels_to_skip,
                tile_address,
                tile_pixel_y,
                tile_pixel_y_offset,
            );
        }
    }

    fn draw_sprite_scanline(&mut self) {
        let tall_sprites = self.lcd_control.contains(LcdControl::SPRITE_SIZE);
        let y_size: u8 = if tall_sprites { 16 } else { 8 };

        // Sort by x such that a lower x-pos will always overwrite a higher x-pos sprite.
        let sprites_to_draw = self
            .oam
            .iter()
            .filter(|sprite| {
                let screen_y_pos = sprite.y_pos as i16 - 16;
                is_sprite_on_scanline(self.current_y as i16, screen_y_pos, y_size as i16)
            })
            .take(10) // Max 10 sprites per scanline
            .sorted_by_key(|x| x.x_pos)
            .rev();

        for sprite in sprites_to_draw {
            // We need to cast to i16 here, as otherwise we'd wrap around when x is f.e 7.
            let screen_x_pos = sprite.x_pos as i16 - 8;
            let screen_y_pos = sprite.y_pos as i16 - 16;

            let x_flip = sprite.attribute_flags.contains(AttributeFlags::X_FLIP);
            let y_flip = sprite.attribute_flags.contains(AttributeFlags::Y_FLIP);
            let is_background_sprite = sprite.attribute_flags.contains(AttributeFlags::OBJ_TO_BG_PRIORITY);

            let mut line = (self.current_y as i16 - screen_y_pos) as u8;

            if y_flip {
                line = y_size - (line + 1);
            }

            let tile_index = sprite.tile_number as usize;
            let tile = if !tall_sprites {
                self.tiles[tile_index]
            } else {
                // If we're on the lower 8x8 block of the 16 pixel tall sprite
                if line < 8 {
                    // Ignore lower bit one
                    self.tiles[tile_index & 0xFE]
                } else {
                    // Add one, if appropriate.
                    // To me an unconditional +1 would make more sense here, however PanDocs
                    // references an OR operation here, so I'll keep it like this for now.
                    self.tiles[tile_index | 0x01]
                }
            };

            let tile_pixel_y = (line as usize % 8) * 8;
            let pixels = tile.get_true_pixel_line(tile_pixel_y);

            for j in 0..=7 {
                // If x is flipped then we want the pixels to go in order to the screen buffer,
                // otherwise it's the reverse.
                let pixel = if x_flip {
                    screen_x_pos + j
                } else {
                    screen_x_pos + (7 - j)
                };

                // Required for the times when sprites are on an x < 8 or y < 16,
                // as parts of those sprites need to then not be rendered.
                // If the BG bit is 1 then the sprite is only drawn if the background colour
                // is color_0, otherwise the background takes precedence.
                if (pixel < 0)
                    || (pixel > 159)
                    || (is_background_sprite && self.scanline_buffer_unpalette[pixel as usize].0 != 0)
                {
                    continue;
                }

                let colour = pixels[j as usize];

                // The colour 0 should be transparent for sprites, therefore we don't draw it.
                if colour != 0x0 {
                    self.scanline_buffer[pixel as usize] = self.get_sprite_palette(sprite).colour(colour);
                    self.scanline_buffer_unpalette[pixel as usize] = (colour, false);
                }
            }
        }
    }

    /// Draw a tile in a way appropriate for both the window, as well as the background.
    /// `pixels_to_skip` will skip pixels so long as it's greater than 0
    fn draw_background_window_line(
        &mut self,
        pixel_counter: &mut i16,
        pixels_to_skip: &mut u8,
        tile_address: usize,
        tile_line_y: usize,
        tile_pixel_y_offset: usize,
    ) {
        // If we can draw 8 pixels in one go, we should.
        // pixel_counter Should be less than 152 otherwise we'd go over the 160 allowed pixels.
        if *pixels_to_skip == 0 && *pixel_counter < 152 {
            self.draw_contiguous_bg_window_block(*pixel_counter as usize, tile_address, tile_line_y);
            *pixel_counter += 8;
        } else {
            let tile = &self.tiles[tile_address];
            for j in (tile_line_y..=tile_pixel_y_offset).rev() {
                // We have to render a partial tile, so skip the first pixels_to_skip and render the rest.
                if *pixels_to_skip > 0 {
                    *pixels_to_skip -= 1;
                    continue;
                }
                // We've exceeded the amount we need to draw, no need to do anything more.
                if *pixel_counter > 159 {
                    break;
                }
                let colour = tile.get_pixel(j);
                self.scanline_buffer[*pixel_counter as usize] = self.bg_window_palette.colour(colour);
                self.scanline_buffer_unpalette[*pixel_counter as usize] = (colour, false);
                *pixel_counter += 1;
            }
        }
    }

    /// This function will immediately draw 8 pixels, skipping several checks and manual
    /// get_pixel_calls().
    #[inline(always)]
    fn draw_contiguous_bg_window_block(&mut self, pixel_counter: usize, tile_address: usize, tile_line_y: usize) {
        let mut tile = &self.tiles[tile_address];
        let colour0 = tile.get_pixel(tile_line_y);
        let colour1 = tile.get_pixel(tile_line_y + 1);
        let colour2 = tile.get_pixel(tile_line_y + 2);
        let colour3 = tile.get_pixel(tile_line_y + 3);
        let colour4 = tile.get_pixel(tile_line_y + 4);
        let colour5 = tile.get_pixel(tile_line_y + 5);
        let colour6 = tile.get_pixel(tile_line_y + 6);
        let colour7 = tile.get_pixel(tile_line_y + 7);
        self.scanline_buffer[pixel_counter + 7] = self.bg_window_palette.colour(colour0);
        self.scanline_buffer[pixel_counter + 6] = self.bg_window_palette.colour(colour1);
        self.scanline_buffer[pixel_counter + 5] = self.bg_window_palette.colour(colour2);
        self.scanline_buffer[pixel_counter + 4] = self.bg_window_palette.colour(colour3);
        self.scanline_buffer[pixel_counter + 3] = self.bg_window_palette.colour(colour4);
        self.scanline_buffer[pixel_counter + 2] = self.bg_window_palette.colour(colour5);
        self.scanline_buffer[pixel_counter + 1] = self.bg_window_palette.colour(colour6);
        self.scanline_buffer[pixel_counter] = self.bg_window_palette.colour(colour7);

        self.scanline_buffer_unpalette[pixel_counter + 7] = (colour0, false);
        self.scanline_buffer_unpalette[pixel_counter + 6] = (colour1, false);
        self.scanline_buffer_unpalette[pixel_counter + 5] = (colour2, false);
        self.scanline_buffer_unpalette[pixel_counter + 4] = (colour3, false);
        self.scanline_buffer_unpalette[pixel_counter + 3] = (colour4, false);
        self.scanline_buffer_unpalette[pixel_counter + 2] = (colour5, false);
        self.scanline_buffer_unpalette[pixel_counter + 1] = (colour6, false);
        self.scanline_buffer_unpalette[pixel_counter] = (colour7, false);
    }

    fn get_sprite_palette(&self, sprite: &SpriteAttribute) -> Palette {
        if !sprite.attribute_flags.contains(AttributeFlags::PALETTE_NUMBER) {
            self.oam_palette_0
        } else {
            self.oam_palette_1
        }
    }

    fn get_tile_address_bg(&self, address: u16) -> u8 {
        if !self.lcd_control.contains(LcdControl::BG_TILE_MAP_SELECT) {
            self.tile_map_9800.data[address as usize]
        } else {
            self.tile_map_9c00.data[address as usize]
        }
    }

    fn get_tile_address_window(&self, address: u16) -> u8 {
        if !self.lcd_control.contains(LcdControl::WINDOW_MAP_SELECT) {
            self.tile_map_9800.data[address as usize]
        } else {
            self.tile_map_9c00.data[address as usize]
        }
    }

    fn ly_lyc_compare(&mut self, interrupts: &mut Interrupts) {
        self.lcd_status.set(LcdStatus::COINCIDENCE_FLAG, self.current_y == self.lyc_compare);
        self.request_stat_interrupt(interrupts);
    }

    pub fn frame_buffer(&self) -> &[RGB; FRAMEBUFFER_SIZE] {
        &self.frame_buffer
    }
}

/// Initialises BG0, OBJ0, OBJ1 in the CGB palettes to `dmg_display_colour` while leaving
/// the remaining palettes default. See PPU `new()` for an explanation as to why.
fn initialise_cgb_palette(
    bg_display: DisplayColour,
    sp0_display: DisplayColour,
    sp1_display: DisplayColour,
) -> ([CgbPalette; 8], [CgbPalette; 8]) {
    fn set_cgb_palette(p: &mut CgbPalette, dmg_display_colour: DisplayColour) {
        for (i, colour) in p.colours.iter_mut().enumerate() {
            colour.rgb = dmg_display_colour.get_colour(i);
        }
    }
    let mut bg_palette = [CgbPalette::default(); 8];
    let mut sprite_palette = [CgbPalette::default(); 8];
    set_cgb_palette(&mut bg_palette[0], bg_display);
    set_cgb_palette(&mut sprite_palette[0], sp0_display);
    set_cgb_palette(&mut sprite_palette[1], sp1_display);

    (bg_palette, sprite_palette)
}

fn is_sprite_on_scanline(scanline_y: i16, y_pos: i16, y_size: i16) -> bool {
    (scanline_y >= y_pos) && (scanline_y < (y_pos + y_size))
}
