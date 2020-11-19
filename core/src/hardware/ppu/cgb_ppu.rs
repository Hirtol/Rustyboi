use crate::hardware::ppu::cgb_vram::CgbTileAttribute;
use crate::hardware::ppu::palette::{DisplayColour, RGB};
use crate::hardware::ppu::register_flags::{AttributeFlags, LcdControl};
use crate::hardware::ppu::tiledata::{Tile, BACKGROUND_TILE_SIZE};
///! This module is purely for CGB specific rendering, look in ppu/mod.rs for DMG mode rendering.
use crate::hardware::ppu::{is_sprite_on_scanline, PPU, RESOLUTION_WIDTH, RGB_CHANNELS};
use itertools::Itertools;
use num_integer::Integer;

impl PPU {
    pub fn draw_cgb_scanline(&mut self) {
        // As soon as wy == yc ANYWHERE in the frame, the window will be considered
        // triggered for the remainder of the frame, and thus can only be disabled
        // if LCD Control WINDOW_DISPlAY is reset.
        // This trigger can happen even if the WINDOW_DISPLAY bit is not set.
        if !self.window_triggered {
            self.window_triggered = self.current_y == self.window_y;
        }

        if self.lcd_control.contains(LcdControl::WINDOW_DISPLAY) {
            if !self.window_triggered || self.window_x > 7 {
                self.draw_cgb_bg_scanline();
            }
            self.draw_cgb_window_scanline();
        } else {
            self.draw_cgb_bg_scanline()
        }

        if self.lcd_control.contains(LcdControl::SPRITE_DISPLAY_ENABLE) {
            self.draw_cgb_sprite_scanline();
        }

        let current_address: usize = self.current_y as usize * RESOLUTION_WIDTH;

        // Copy the value of the current scanline to the framebuffer.
        self.frame_buffer[current_address..current_address + RESOLUTION_WIDTH].copy_from_slice(&self.scanline_buffer);
    }

    fn draw_cgb_bg_scanline(&mut self) {
        let scanline_to_be_rendered = self.current_y.wrapping_add(self.scroll_y);
        // scanline_to_be_rendered can be in range 0-255, where each tile is 8 in length.
        // As we'll want to use this variable to index on the TileMap (1 byte pointer to tile)
        // We need to first divide by 8, to then multiply by 32 for our array (32*32) with a 1d representation.
        let tile_lower_bound = ((scanline_to_be_rendered / 8) as u16 * 32) + (self.scroll_x / 8) as u16;
        // 20 since 20*8 = 160 pixels
        let mut tile_higher_bound = (tile_lower_bound + 20);

        // Which particular y coordinate to use from an 8x8 tile.
        let tile_line_y = scanline_to_be_rendered as usize % 8;
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
            let tile_attributes = self.get_tile_attributes_cgb_bg(i % BACKGROUND_TILE_SIZE as u16);
            // We always add an offset in case we're supposed to look in VRAM bank 1.
            let mut tile_address = tile_relative_address
                + (384 * tile_attributes.contains(CgbTileAttribute::TILE_VRAM_BANK_NUMBER) as usize);

            self.draw_cgb_background_window_line(
                &mut pixel_counter,
                &mut pixels_to_skip,
                tile_address,
                tile_relative_address,
                tile_line_y,
                tile_attributes,
            )
        }
    }

    fn draw_cgb_window_scanline(&mut self) {
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

        let tile_line_y = self.window_counter as usize % 8;

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
            let tile_attributes = self.get_tile_attributes_cgb_window(i);
            let mut tile_address = tile_relative_address
                + (384 * tile_attributes.contains(CgbTileAttribute::TILE_VRAM_BANK_NUMBER) as usize);

            self.draw_cgb_background_window_line(
                &mut pixel_counter,
                &mut pixels_to_skip,
                tile_address,
                tile_relative_address,
                tile_line_y,
                tile_attributes,
            );
        }
    }

    fn draw_cgb_sprite_scanline(&mut self) {
        let tall_sprites = self.lcd_control.contains(LcdControl::SPRITE_SIZE);
        let y_size: u8 = if tall_sprites { 16 } else { 8 };
        let always_display_sprite = !self.lcd_control.contains(LcdControl::BG_WINDOW_PRIORITY);

        let sprites_to_draw = self
            .oam
            .iter()
            .filter(|sprite| {
                let screen_y_pos = sprite.y_pos as i16 - 16;
                is_sprite_on_scanline(self.current_y as i16, screen_y_pos, y_size as i16)
            })
            .take(10)
            .collect_vec(); // Max 10 sprites per scanline

        // Need to reverse here since we can't take rev() after take() :(
        // We reverse since the CGB sorts based on sprite position in OAM.
        for sprite in sprites_to_draw.into_iter().rev() {
            let screen_x_pos = sprite.x_pos as i16 - 8;
            let screen_y_pos = sprite.y_pos as i16 - 16;

            let x_flip = sprite.attribute_flags.contains(AttributeFlags::X_FLIP);
            let y_flip = sprite.attribute_flags.contains(AttributeFlags::Y_FLIP);
            let is_background_sprite = sprite.attribute_flags.contains(AttributeFlags::OBJ_TO_BG_PRIORITY);

            let mut line = (self.current_y as i16 - screen_y_pos) as u8;

            if y_flip {
                line = y_size - (line + 1);
            }

            let tile_index = sprite.tile_number as usize
                + (384 * sprite.attribute_flags.contains(AttributeFlags::TILE_VRAM_BANK) as usize);
            let tile = if !tall_sprites {
                self.tiles[tile_index]
            } else {
                // If we're on the lower 8x8 block of the 16 pixel tall sprite
                if line < 8 {
                    // Ignore lower bit one
                    self.tiles[tile_index & 0xFFFE]
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
                if !always_display_sprite {
                    if (pixel < 0)
                        || (pixel > 159)
                        || ((is_background_sprite || self.scanline_buffer_unpalette[pixel as usize].1)
                            && self.scanline_buffer_unpalette[pixel as usize].0 != 0)
                    {
                        continue;
                    }
                }

                let colour = pixels[j as usize];

                // The colour 0 should be transparent for sprites, therefore we don't draw it.
                if colour != 0x0 {
                    self.scanline_buffer[pixel as usize] = self.cgb_sprite_palette
                        [sprite.attribute_flags.get_cgb_palette_number()]
                    .colours[colour as usize]
                        .rgb;
                    self.scanline_buffer_unpalette[pixel as usize] = (colour, false);
                }
            }
        }
    }

    /// Draw a tile in a way appropriate for both the window, as well as the background.
    /// `pixels_to_skip` will skip pixels so long as it's greater than 0
    fn draw_cgb_background_window_line(
        &mut self,
        pixel_counter: &mut i16,
        pixels_to_skip: &mut u8,
        mut tile_address: usize,
        tile_relative_address: usize,
        tile_line_y: usize,
        tile_attributes: CgbTileAttribute,
    ) {
        // If we've selected the 8800-97FF mode we need to add a 256 offset, and then
        // add/subtract the relative address. (since we can then reach tiles 128-384)
        if !self.lcd_control.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
            tile_address = (256_usize).wrapping_add((tile_relative_address as i8) as usize)
                + (384 * tile_attributes.contains(CgbTileAttribute::TILE_VRAM_BANK_NUMBER) as usize);
        }

        let tile_pixel_y = if tile_attributes.contains(CgbTileAttribute::Y_FLIP) {
            (7 - tile_line_y) * 8
        } else {
            tile_line_y * 8
        };
        // If we can draw 8 pixels in one go, we should.
        // pixel_counter Should be less than 152 otherwise we'd go over the 160 allowed pixels.
        if *pixels_to_skip == 0 && *pixel_counter < 152 {
            self.draw_cgb_contiguous_bg_window_block(
                *pixel_counter as usize,
                tile_address,
                tile_pixel_y,
                tile_attributes,
            );
            *pixel_counter += 8;
        } else {
            let x_flip = tile_attributes.contains(CgbTileAttribute::X_FLIP);
            let bg_priority = tile_attributes.contains(CgbTileAttribute::BG_TO_OAM_PRIORITY);
            let tile_pixel_y_offset = tile_pixel_y + 7;
            let tile = &self.tiles[tile_address];
            // Yes this is ugly, yes this means a vtable call, yes I'd like to do it differently.
            // Only other way is to duplicate the for loop since the .rev() is a different iterator.
            let iterator: Box<dyn Iterator<Item = usize>> = if x_flip {
                Box::new((tile_pixel_y..=tile_pixel_y_offset))
            } else {
                Box::new((tile_pixel_y..=tile_pixel_y_offset).rev())
            };

            for j in iterator {
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
                self.scanline_buffer[*pixel_counter as usize] = self.cgb_bg_palette[tile_attributes.bg_palette_numb()].colour(colour);
                self.scanline_buffer_unpalette[*pixel_counter as usize] = (colour, bg_priority);
                *pixel_counter += 1;
            }
        }
    }

    /// This function will immediately draw 8 pixels, skipping several checks and manual
    /// get_pixel_calls().
    #[inline(always)]
    fn draw_cgb_contiguous_bg_window_block(
        &mut self,
        pixel_counter: usize,
        tile_address: usize,
        tile_line_y: usize,
        tile_attributes: CgbTileAttribute,
    ) {
        let tile = &self.tiles[tile_address];
        let palette = self.cgb_bg_palette[tile_attributes.bg_palette_numb()];
        let bg_priority = tile_attributes.contains(CgbTileAttribute::BG_TO_OAM_PRIORITY);

        let colour0 = tile.get_pixel(tile_line_y);
        let colour1 = tile.get_pixel(tile_line_y + 1);
        let colour2 = tile.get_pixel(tile_line_y + 2);
        let colour3 = tile.get_pixel(tile_line_y + 3);
        let colour4 = tile.get_pixel(tile_line_y + 4);
        let colour5 = tile.get_pixel(tile_line_y + 5);
        let colour6 = tile.get_pixel(tile_line_y + 6);
        let colour7 = tile.get_pixel(tile_line_y + 7);

        if tile_attributes.contains(CgbTileAttribute::X_FLIP) {
            self.scanline_buffer[pixel_counter] = palette.colour(colour0);
            self.scanline_buffer[pixel_counter + 1] = palette.colour(colour1);
            self.scanline_buffer[pixel_counter + 2] = palette.colour(colour2);
            self.scanline_buffer[pixel_counter + 3] = palette.colour(colour3);
            self.scanline_buffer[pixel_counter + 4] = palette.colour(colour4);
            self.scanline_buffer[pixel_counter + 5] = palette.colour(colour5);
            self.scanline_buffer[pixel_counter + 6] = palette.colour(colour6);
            self.scanline_buffer[pixel_counter + 7] = palette.colour(colour7);
            // We know the colourX will be a u8, we just preemptively made it a usize for index convenience.
            self.scanline_buffer_unpalette[pixel_counter] = (colour0, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 1] = (colour1, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 2] = (colour2, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 3] = (colour3, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 4] = (colour4, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 5] = (colour5, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 6] = (colour6, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 7] = (colour7, bg_priority);
        } else {
            self.scanline_buffer[pixel_counter + 7] = palette.colour(colour0);
            self.scanline_buffer[pixel_counter + 6] = palette.colour(colour1);
            self.scanline_buffer[pixel_counter + 5] = palette.colour(colour2);
            self.scanline_buffer[pixel_counter + 4] = palette.colour(colour3);
            self.scanline_buffer[pixel_counter + 3] = palette.colour(colour4);
            self.scanline_buffer[pixel_counter + 2] = palette.colour(colour5);
            self.scanline_buffer[pixel_counter + 1] = palette.colour(colour6);
            self.scanline_buffer[pixel_counter] = palette.colour(colour7);
            // We know the colourX will be a u8, we just preemptively made it a usize for index convenience.
            self.scanline_buffer_unpalette[pixel_counter + 7] = (colour0, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 6] = (colour1, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 5] = (colour2, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 4] = (colour3, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 3] = (colour4, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 2] = (colour5, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter + 1] = (colour6, bg_priority);
            self.scanline_buffer_unpalette[pixel_counter] = (colour7, bg_priority);
        }
    }

    fn get_tile_attributes_cgb_bg(&self, address: u16) -> CgbTileAttribute {
        if !self.lcd_control.contains(LcdControl::BG_TILE_MAP_SELECT) {
            self.cgb_9800_tile_map.attributes[address as usize]
        } else {
            self.cgb_9c00_tile_map.attributes[address as usize]
        }
    }

    fn get_tile_attributes_cgb_window(&self, address: u16) -> CgbTileAttribute {
        if !self.lcd_control.contains(LcdControl::WINDOW_MAP_SELECT) {
            self.cgb_9800_tile_map.attributes[address as usize]
        } else {
            self.cgb_9c00_tile_map.attributes[address as usize]
        }
    }
}
