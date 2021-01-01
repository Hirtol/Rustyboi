use itertools::Itertools;
use num_integer::Integer;

use crate::gb_emu::GameBoyModel;
use crate::hardware::ppu::cgb_vram::{CgbPalette, CgbPaletteIndex, CgbTileMap};
use crate::hardware::ppu::palette::{DisplayColour, Palette, RGB};
use crate::hardware::ppu::register_flags::*;
use crate::hardware::ppu::tiledata::*;
use crate::hardware::ppu::Mode::{Hblank, LcdTransfer, OamSearch, Vblank};
use crate::io::interrupts::{InterruptFlags, Interrupts};
use crate::scheduler::{EventType, Scheduler};

pub const RESOLUTION_WIDTH: usize = 160;
pub const RESOLUTION_HEIGHT: usize = 144;
pub const RGB_CHANNELS: usize = 3;
pub const FRAMEBUFFER_SIZE: usize = RESOLUTION_HEIGHT * RESOLUTION_WIDTH;

pub mod cgb_ppu;
pub mod cgb_vram;
pub mod debugging_features;
pub mod dma;
pub mod memory_binds;
pub mod palette;
pub mod register_flags;
pub mod tiledata;
pub mod timing;

#[derive(Debug, PartialOrd, PartialEq, Copy, Clone)]
pub enum Mode {
    Hblank = 0x0,
    Vblank = 0x1,
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
    latest_lcd_transfer_start: u64,
    current_lcd_transfer_duration: u64,
}

impl PPU {
    /// Instantiates a PPU with the provided `DisplayColour`.
    /// The PPU will output a framebuffer with RGB24 values based on the `DisplayColour`
    /// if the emulator is running in `DMG` mode.
    ///
    /// In addition we proceed to load `DisplayColour` into the CGB palette registries (BG0, OBJ0, OBJ1),
    /// and we'll *always* use those three registries as our source of `RGB` colour (Both in `DMG` and `CGB` mode).
    pub fn new(
        bg_display_colour: DisplayColour,
        sp0_display: DisplayColour,
        sp1_display: DisplayColour,
        cgb_rendering: bool,
        gb_model: GameBoyModel,
    ) -> Self {
        let (cgb_bg_palette, cgb_sprite_palette) = if !cgb_rendering {
            initialise_cgb_palette(bg_display_colour, sp0_display, sp1_display)
        } else {
            ([CgbPalette::default(); 8], [CgbPalette::default(); 8])
        };
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
            latest_lcd_transfer_start: 0,
            current_lcd_transfer_duration: 0,
        }
    }

    pub fn increment_current_y(&mut self, interrupts: &mut Interrupts) {
        self.current_y = self.current_y.wrapping_add(1);
        self.ly_lyc_compare(interrupts);
    }

    pub fn oam_search(&mut self, interrupts: &mut Interrupts) {
        // After V-Blank we don't want to trigger the interrupt immediately.
        if self.lcd_status.mode_flag() != Vblank {
            self.increment_current_y(interrupts);
        }

        self.lcd_status.set_mode_flag(Mode::OamSearch);
        // OAM Interrupt
        self.request_stat_interrupt(interrupts);
    }

    pub fn lcd_transfer(&mut self, scheduler: &Scheduler) {
        // Drawing (Mode 3)
        self.latest_lcd_transfer_start = scheduler.current_time;
        self.lcd_status.set_mode_flag(LcdTransfer);

        // Draw our actual line once we enter Drawing mode.
        if self.cgb_rendering {
            self.draw_cgb_scanline();
        } else {
            self.draw_scanline();
        }
    }

    pub fn hblank(&mut self, interrupts: &mut Interrupts) {
        // Since mid scanline palette writes are possible we'll only push the palette
        // pixels after Mode 3.
        self.push_current_scanline_to_framebuffer();
        self.lcd_status.set_mode_flag(Hblank);

        self.request_stat_interrupt(interrupts);
    }

    pub fn vblank(&mut self, interrupts: &mut Interrupts) {
        self.lcd_status.set_mode_flag(Vblank);

        // Check for line 144 lyc.
        self.increment_current_y(interrupts);

        self.window_counter = 0;
        self.window_triggered = false;
        // Check for Vblank flag in LCD Stat
        self.request_stat_interrupt(interrupts);

        interrupts.insert_interrupt(InterruptFlags::VBLANK);
    }

    pub fn vblank_wait(&mut self, interrupts: &mut Interrupts) {
        self.increment_current_y(interrupts);
    }

    /// On line 153 (the line right before we transfer to OAM Search) the LY register only
    /// reads 153 for 4 cycles, this method will set LY to 0 and do the ly_lyc compare.
    pub fn late_y_153_to_0(&mut self, interrupts: &mut Interrupts) {
        self.current_y = 0;
        self.ly_lyc_compare(interrupts);
    }

    /// Since multiple scan-lines might be drawn in a LCD_Transfer mode cycle we'll
    /// only want to copy it to the framebuffer after we're done.
    #[inline]
    fn push_current_scanline_to_framebuffer(&mut self) {
        let current_address: usize = self.current_y as usize * RESOLUTION_WIDTH;
        // Copy the value of the current scanline to the framebuffer.
        self.frame_buffer[current_address..current_address + RESOLUTION_WIDTH].copy_from_slice(&self.scanline_buffer);
    }

    #[inline(always)]
    pub fn draw_scanline(&mut self) {
        if self.cgb_rendering {
            self.draw_cgb_scanline();
        } else {
            self.draw_dmg_scanline();
        }
    }
    #[inline(always)]
    pub fn draw_dmg_scanline(&mut self) {
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
            let bg_colour = self.bg_window_palette.colours[0];
            for pixel in self.scanline_buffer.iter_mut() {
                *pixel = bg_colour;
            }
        }

        if self.lcd_control.contains(LcdControl::SPRITE_DISPLAY_ENABLE) {
            self.draw_sprite_scanline();
        }
    }

    fn draw_bg_scanline(&mut self) {
        let scanline_to_be_rendered = self.current_y.wrapping_add(self.scroll_y);
        let tile_lower_bound = ((scanline_to_be_rendered / 8) as u16 * 32) + (self.scroll_x / 8) as u16;
        let mut tile_higher_bound = tile_lower_bound + 20;

        // Which particular y coordinate to use from an 8x8 tile.
        let tile_pixel_y = (scanline_to_be_rendered as usize % 8) * 8;
        let tile_pixel_y_offset = tile_pixel_y + 7;
        let mut pixels_drawn: i16 = 0;
        // For cases where the scroll x is not nicely aligned on tile boundries.
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
            if (self.scroll_x as u16 + pixels_drawn as u16) > 255 {
                i -= 32;
            }
            // Modulo for the y-wraparound if scroll_y > 111
            let tile_relative_address = self.get_tile_address_bg(i % BACKGROUND_TILE_SIZE as u16) as usize;
            let mut tile_address = tile_relative_address;

            // If we've selected the 8800-97FF mode we need to add a 256 offset, and then
            // add/subtract the relative address. (since we can then reach tiles 128-384)
            if !self.lcd_control.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
                tile_address = (256_usize).wrapping_add((tile_relative_address as i8) as usize);
            }

            self.draw_background_window_line(
                &mut pixels_drawn,
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
        let window_x = (self.window_x as i16).wrapping_sub(7);
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
        let tile_pixel_y_offset = tile_pixel_y + 7;

        // If window is less than 0 we want to skip those amount of pixels, otherwise we render as normal.
        // This means that we must take the absolute value of window_x for the pixels_skip, therefore the -
        let (mut pixels_drawn, mut pixels_to_skip) = if window_x >= 0 {
            (window_x, 0)
        } else {
            (0, (-window_x) as u8)
        };

        self.window_counter += 1;

        for i in tile_lower_bound..tile_higher_bound {
            let tile_relative_address = self.get_tile_address_window(i) as usize;
            let mut tile_address = tile_relative_address;

            // If we've selected the 8800-97FF mode we need to add a 256 offset, and then
            // add/subtract the relative address.
            if !self.lcd_control.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
                tile_address = (256_usize).wrapping_add((tile_relative_address as i8) as usize);
            }

            self.draw_background_window_line(
                &mut pixels_drawn,
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
                if line < 8 {
                    self.tiles[tile_index & 0xFE]
                } else {
                    self.tiles[tile_index | 0x01]
                }
            };

            let tile_pixel_y = (line as usize % 8) * 8;
            let pixels = tile.get_true_pixel_line(tile_pixel_y);

            for j in 0..=7 {
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

                // The colour 0 should be transparent for sprites.
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
        pixels_drawn: &mut i16,
        pixels_to_skip: &mut u8,
        tile_address: usize,
        tile_line_y: usize,
        tile_pixel_y_offset: usize,
    ) {
        // If we can draw 8 pixels in one go, we should.
        // pixel_counter Should be less than 152 otherwise we'd go over the 160 allowed pixels.
        if *pixels_to_skip == 0 && *pixels_drawn < 152 {
            self.draw_contiguous_bg_window_block(*pixels_drawn as usize, tile_address, tile_line_y);
            *pixels_drawn += 8;
        } else {
            let tile = &self.tiles[tile_address];
            for j in (tile_line_y..=tile_pixel_y_offset).rev() {
                // We have to render a partial tile, so skip the first pixels_to_skip and render the rest.
                if *pixels_to_skip > 0 {
                    *pixels_to_skip -= 1;
                    continue;
                }
                // We've exceeded the amount we need to draw, no need to do anything more.
                if *pixels_drawn > 159 {
                    break;
                }
                let colour = tile.get_pixel(j);
                self.scanline_buffer[*pixels_drawn as usize] = self.bg_window_palette.colour(colour);
                self.scanline_buffer_unpalette[*pixels_drawn as usize] = (colour, false);
                *pixels_drawn += 1;
            }
        }
    }

    /// This function will immediately draw 8 pixels, skipping several checks and manual
    /// get_pixel_calls().
    #[inline(always)]
    fn draw_contiguous_bg_window_block(&mut self, pixels_drawn: usize, tile_address: usize, tile_line_y: usize) {
        let tile = &self.tiles[tile_address];
        let pixel_line = tile.get_true_pixel_line(tile_line_y);

        for (i, colour) in pixel_line.iter().rev().copied().enumerate() {
            let index = pixels_drawn + i;
            self.scanline_buffer[index] = self.bg_window_palette.colour(colour);
            self.scanline_buffer_unpalette[index] = (colour, false);
        }
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
