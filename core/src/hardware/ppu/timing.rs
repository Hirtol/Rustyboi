use crate::hardware::ppu::{PPU, is_sprite_on_scanline};
use crate::hardware::ppu::register_flags::LcdControl;
use crate::scheduler::{Scheduler, Event, EventType};
use crate::io::interrupts::Interrupts;
use std::env::Args;
use crate::hardware::mmu::Memory;

pub const SCANLINE_DURATION: u64 = 456;
pub const BASE_LCD_TRANSFER_DURATION: u64 = 172;
pub const BASE_HBLANK_DURATION: u64 = 204;
pub const OAM_SEARCH_DURATION: u64 = 80;

impl PPU {

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

    /// Roughly calculates the expected duration of LCD transfer (mode 3)
    /// This is not entirely accurate yet, as I'm not sure about the sprite timings.
    #[inline]
    fn calculate_lcd_transfer_duration(&self) -> u64 {
        // All cycles mentioned here are t-cycles
        let mut base_cycles = BASE_LCD_TRANSFER_DURATION;
        // If we need to skip a few initial pixels this scanline.
        base_cycles += (self.scroll_x % 8) as u64;

        // If there's an active window the fifo pauses for *at least* 6 cycles.
        if self.window_triggered && self.window_x < 168 && self.lcd_control.contains(LcdControl::WINDOW_DISPLAY) {
            base_cycles += 6;
        }

        let y_size: u8 = if self.lcd_control.contains(LcdControl::SPRITE_SIZE) { 16 } else { 8 };

        base_cycles += self.oam.iter()
            .filter(|sprite| {
                let screen_y_pos = sprite.y_pos as i16 - 16;
                is_sprite_on_scanline(self.current_y as i16, screen_y_pos, y_size as i16)
            })
            .take(10)
            .count() as u64 * 6;

        base_cycles
    }

}