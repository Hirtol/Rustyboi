use crate::emulator::GameBoyModel;
use crate::hardware::ppu::cgb_vram::CgbPalette;
use crate::hardware::ppu::palette::{DisplayColour, Palette, RGB};
use crate::hardware::ppu::tiledata::Tile;
use crate::hardware::ppu::PPU;
use bitflags::_core::iter::FromIterator;

impl PPU {
    /// Returns an array of the full 768 tiles rendered next to each other in a
    /// 128 * 384 RGB pixel array. (16 tiles per line)
    pub fn tiles_cgb(&self) -> [RGB; 49152] {
        let mut res = [RGB::default(); 49152];
        // To be multiplied by 8 since it counts tiles.
        for current_tile_line in 0..48 {
            let tile_floor = current_tile_line * 16;
            let tile_ceil = tile_floor + 16;

            for (tile_in_row, tile) in self.tiles[tile_floor..tile_ceil].iter().enumerate() {
                let rendered_tile = self.render_tile(tile);

                for (index, j) in rendered_tile.iter().enumerate() {
                    let selected_line = (current_tile_line * 8) + (index / 8);
                    let tile_index_in_row = tile_in_row * 8;
                    res[(selected_line * 128) + tile_index_in_row + (index % 8)] = *j;
                }
            }
        }

        res
    }

    fn render_tile(&self, tile: &Tile) -> [RGB; 64] {
        let mut result = [RGB::default(); 64];
        let mut pixel_counter = 0;
        for _ in 0..8 {
            let colour0 = tile.get_pixel(pixel_counter);
            let colour1 = tile.get_pixel(pixel_counter + 1);
            let colour2 = tile.get_pixel(pixel_counter + 2);
            let colour3 = tile.get_pixel(pixel_counter + 3);
            let colour4 = tile.get_pixel(pixel_counter + 4);
            let colour5 = tile.get_pixel(pixel_counter + 5);
            let colour6 = tile.get_pixel(pixel_counter + 6);
            let colour7 = tile.get_pixel(pixel_counter + 7);
            //TODO: Add palette prediction by using tile maps
            result[pixel_counter + 7] = self.bg_window_palette.colour(colour0);
            result[pixel_counter + 6] = self.bg_window_palette.colour(colour1);
            result[pixel_counter + 5] = self.bg_window_palette.colour(colour2);
            result[pixel_counter + 4] = self.bg_window_palette.colour(colour3);
            result[pixel_counter + 3] = self.bg_window_palette.colour(colour4);
            result[pixel_counter + 2] = self.bg_window_palette.colour(colour5);
            result[pixel_counter + 1] = self.bg_window_palette.colour(colour6);
            result[pixel_counter] = self.bg_window_palette.colour(colour7);
            pixel_counter += 8;
        }

        result
    }
}

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct PaletteDebugInfo {
    pub bg_palette: Vec<[RGB; 4]>,
    pub sprite_palette: Vec<[RGB; 4]>,
}

impl PaletteDebugInfo {
    pub fn new(ppu: &PPU, current_mode: GameBoyModel) -> Self {
        let mut bg_palette;
        let mut sprite_palette;
        if current_mode.is_dmg() {
            bg_palette = vec![[RGB::default(); 4]; 8];
            sprite_palette = vec![[RGB::default(); 4]; 8];
            bg_palette[0] = ppu.bg_window_palette.colours;
            sprite_palette[0] = ppu.oam_palette_0.colours;
            sprite_palette[1] = ppu.oam_palette_1.colours;
        } else {
            let cgb_to_rgb_array = |cgb: [CgbPalette; 8]| {
                cgb.iter()
                    .map(|p| p.colours.iter().map(|c| c.rgb).collect())
                    .collect::<Vec<[RGB; 4]>>()
            };
            bg_palette = cgb_to_rgb_array(ppu.cgb_bg_palette);
            sprite_palette = cgb_to_rgb_array(ppu.cgb_sprite_palette);
        }

        PaletteDebugInfo {
            bg_palette,
            sprite_palette,
        }
    }
}

impl Default for PaletteDebugInfo {
    fn default() -> Self {
        let bg_palette = vec![[RGB::default(); 4]; 8];
        let sprite_palette = vec![[RGB::default(); 4]; 8];
        PaletteDebugInfo {
            bg_palette,
            sprite_palette,
        }
    }
}

impl FromIterator<RGB> for [RGB; 4] {
    fn from_iter<T: IntoIterator<Item = RGB>>(iter: T) -> Self {
        let mut result = [RGB::default(); 4];
        for (i, rgb) in iter.into_iter().enumerate() {
            result[i] = rgb;
        }
        result
    }
}
