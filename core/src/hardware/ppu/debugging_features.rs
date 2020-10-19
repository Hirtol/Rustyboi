use crate::hardware::ppu::PPU;
use crate::hardware::ppu::tiledata::Tile;
use crate::hardware::ppu::palette::{RGB, DisplayColour};

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
                    res[(selected_line*128) + tile_index_in_row + (index % 8)] = *j;
                }
            }
        }

        res
    }

    fn render_tile(&self, tile: &Tile) -> [RGB; 64] {
        let mut result = [RGB::default(); 64];
        let mut pixel_counter = 0;
        for i in 0..8 {
            let (top, bottom) = tile.get_pixel_line(i);

            let top_pixel_data = top as usize;
            let bottom_pixel_data = bottom as usize;

            let colour0 = top_pixel_data & 0x1 | ((bottom_pixel_data & 0x1) << 1) ;
            let colour1 = (top_pixel_data & 0x2) >> 1 | (bottom_pixel_data & 0x2);
            let colour2 = (top_pixel_data & 4) >> 2 | ((bottom_pixel_data & 4) >> 1);
            let colour3 = (top_pixel_data & 8) >> 3 | ((bottom_pixel_data & 8) >> 2);
            let colour4 = (top_pixel_data & 16) >> 4 | ((bottom_pixel_data & 16) >> 3);
            let colour5 = (top_pixel_data & 32) >> 5 | ((bottom_pixel_data & 32) >> 4);
            let colour6 = (top_pixel_data & 64) >> 6 | ((bottom_pixel_data & 64) >> 5);
            let colour7 = (top_pixel_data & 128) >> 7 | ((bottom_pixel_data & 128) >> 6);
            //TODO: Add palette prediction by using tile maps
            result[pixel_counter + 7] = self.bg_window_palette.colours[colour0];
            result[pixel_counter + 6] = self.bg_window_palette.colours[colour1];
            result[pixel_counter + 5] = self.bg_window_palette.colours[colour2];
            result[pixel_counter + 4] = self.bg_window_palette.colours[colour3];
            result[pixel_counter + 3] = self.bg_window_palette.colours[colour4];
            result[pixel_counter + 2] = self.bg_window_palette.colours[colour5];
            result[pixel_counter + 1] = self.bg_window_palette.colours[colour6];
            result[pixel_counter] = self.bg_window_palette.colours[colour7];
            pixel_counter += 8;
        }

        result
    }
}