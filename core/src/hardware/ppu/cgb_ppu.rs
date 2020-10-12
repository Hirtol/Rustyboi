///! This module is purely for CGB specific rendering, look in ppu/mod.rs for DMG mode rendering.
use crate::hardware::ppu::{PPU, RESOLUTION_WIDTH};
use crate::hardware::ppu::register_flags::LcdControl;

impl PPU {
    fn draw_cgb_scanline(&mut self) {
        // As soon as wy == yc ANYWHERE in the frame, the window will be considered
        // triggered for the remainder of the frame, and thus can only be disabled
        // if LCD Control WINDOW_DISPlAY is reset.
        // This trigger can happen even if the WINDOW_DISPLAY bit is not set.
        if !self.window_triggered {
            self.window_triggered = self.current_y == self.window_y;
        }
        //TODO: Note, BG_WINDOW_PRIORITY has a different meaning for CGB!
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
            let bgcolour = self.bg_window_palette.color_0();
            for pixel in self.scanline_buffer.iter_mut() {
                *pixel = bgcolour;
            }
        }

        if self.lcd_control.contains(LcdControl::SPRITE_DISPLAY_ENABLE) {
            self.draw_sprite_scanline();
        }

        let current_address: usize = self.current_y as usize * RESOLUTION_WIDTH;

        // Copy the value of the current scanline to the framebuffer.
        self.frame_buffer[current_address..current_address + RESOLUTION_WIDTH].copy_from_slice(&self.scanline_buffer);
    }
}