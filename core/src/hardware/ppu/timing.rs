use crate::hardware::ppu::{PPU, is_sprite_on_scanline};
use crate::hardware::ppu::register_flags::LcdControl;
use crate::scheduler::{Scheduler, Event, EventType};
use crate::io::interrupts::Interrupts;
use std::env::Args;
use crate::hardware::mmu::Memory;
use crate::hardware::ppu::Mode::LcdTransfer;
use itertools::Itertools;
use crate::hardware::ppu::tiledata::SpriteAttribute;

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

    /// Emulate a pixel FIFO by synchronising the state after a mid-scanline write.
    /// If that happens we'll just rerender the scanline from the point we should've been at
    /// in this clock-cycle, with the new palette in place.
    ///
    /// Since CGB palettes are locked during mode 3 we don't have to worry about those mid-scanline
    /// writes.
    pub fn handle_mid_scanline_palette(&mut self, scheduler: &mut Scheduler) {
        if self.get_current_mode() != LcdTransfer || self.cgb_rendering {
            return;
        }
        let cycles_passed = scheduler.current_time - self.latest_lcd_transfer_start;
        // First 12 cycles are ignored.
        if cycles_passed <= 12 {
            self.draw_scanline();
            return;
        }
        let to_skip = (self.scroll_x % 8) as u64;
        let mut current_scanline = self.scanline_buffer.clone();
        let mut pixels_drawn = (cycles_passed.saturating_sub(12 + to_skip)) as usize;

        // If there are no special events (sprites or window) then it's safe to assume
        // 1 cycle == 1 pixel. Otherwise we do everything down below.
        if self.current_lcd_transfer_duration-to_skip != 172 {
            //TODO: Fix the fact that mid-scanline writes to scroll x or window will have effects here
            // and cause crashing in Road Rash if we didn't use min(159, value).
            let mut actual_pixels_drawn = 0;
            let mut cycles_to_go = pixels_drawn;
            let window_trigger = self.window_triggered && self.window_x < 168 && self.lcd_control.contains(LcdControl::WINDOW_DISPLAY);
            let window_x = if window_trigger { (self.window_x as i16).saturating_sub(7) } else { 255 };
            let mut sprites_to_draw = self.get_drawable_sprites().collect_vec();

            while cycles_to_go > 0 {
                if let Some(sprite) = sprites_to_draw.last() {
                    if sprite.x_pos >= actual_pixels_drawn {
                        cycles_to_go = cycles_to_go.saturating_sub(6);
                        sprites_to_draw.pop();
                    }
                }

                actual_pixels_drawn += 1;
                cycles_to_go = cycles_to_go.saturating_sub(1);
            }

            if actual_pixels_drawn as i16 >= window_x {
                actual_pixels_drawn -= actual_pixels_drawn.saturating_sub(6);
            }

            pixels_drawn = std::cmp::min(actual_pixels_drawn as usize, 159);
        }

        self.draw_scanline();
        self.scanline_buffer[..pixels_drawn].swap_with_slice(&mut current_scanline[..pixels_drawn]);
    }

    fn get_drawable_sprites(&self) -> impl Iterator<Item=&SpriteAttribute> {
        let y_size: u8 = if self.lcd_control.contains(LcdControl::SPRITE_SIZE) { 16 } else { 8 };
        self
            .oam
            .iter()
            .filter(|sprite| {
                let screen_y_pos = sprite.y_pos as i16 - 16;
                is_sprite_on_scanline(self.current_y as i16, screen_y_pos, y_size as i16)
            })
            .take(10) // Max 10 sprites per scanline
            .sorted_by_key(|x| x.x_pos)
            .rev()
    }

}