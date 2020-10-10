//! This module is purely used for providing access to PPU memory resources
//! to the MMU.
use crate::hardware::mmu::{OAM_ATTRIBUTE_START, INVALID_READ};
use crate::hardware::ppu::PPU;
use crate::print_array_raw;
use crate::scheduler::EventType::{HBLANK, VBLANK, DMATransferComplete};

use super::*;

impl PPU {
    pub fn get_tile_byte(&self, address: u16) -> u8 {
        let (tile_address, byte_address) = get_tile_address(address);

        self.tiles[tile_address].data[byte_address]
    }

    pub fn set_tile_byte(&mut self, address: u16, value: u8) {
        let (tile_address, byte_address) = get_tile_address(address);

        self.tiles[tile_address].data[byte_address] = value;
    }

    pub fn get_tilemap_byte(&self, address: u16) -> u8 {
        match address {
            TILEMAP_9800_START..=TILEMAP_9800_END => self.tile_map_9800.data[(address - TILEMAP_9800_START) as usize],
            // 9C00, assuming no malicious calls
            _ => self.tile_map_9c00.data[(address - TILEMAP_9C00_START) as usize],
        }
    }

    pub fn set_tilemap_byte(&mut self, address: u16, value: u8) {
        match address {
            TILEMAP_9800_START..=TILEMAP_9800_END => {
                self.tile_map_9800.data[(address - TILEMAP_9800_START) as usize] = value
            }
            // 9C00, assuming no malicious calls
            _ => self.tile_map_9c00.data[(address - TILEMAP_9C00_START) as usize] = value,
        }
    }

    pub fn get_oam_byte(&self, address: u16) -> u8 {
        if self.lcd_status.mode_flag() != OamSearch && self.lcd_status.mode_flag() != LcdTransfer && !self.oam_transfer_ongoing {
            let relative_address = (address - OAM_ATTRIBUTE_START) / 4;

            self.oam[relative_address as usize].get_byte((address % 4) as u8)
        } else {
            log::info!("Attempted read of blocked OAM, transfer ongoing: {}", self.oam_transfer_ongoing);
            INVALID_READ
        }
    }

    pub fn set_oam_byte(&mut self, address: u16, value: u8) {
        if self.lcd_status.mode_flag() != OamSearch && self.lcd_status.mode_flag() != LcdTransfer && !self.oam_transfer_ongoing {
            let relative_address = (address - OAM_ATTRIBUTE_START) / 4;

            self.oam[relative_address as usize].set_byte((address % 4) as u8, value);
        }
    }

    pub fn get_lcd_control(&self) -> u8 {
        self.lcd_control.bits()
    }

    pub fn get_lcd_status(&self) -> u8 {
        self.lcd_status.bits()
    }

    pub fn get_scy(&self) -> u8 {
        self.scroll_y
    }

    pub fn get_scx(&self) -> u8 {
        self.scroll_x
    }

    pub fn get_ly(&self) -> u8 {
        self.current_y
    }

    pub fn get_lyc(&self) -> u8 {
        self.compare_line
    }

    pub fn get_bg_palette(&self) -> u8 {
        self.bg_window_palette.into()
    }

    pub fn get_oam_palette_0(&self) -> u8 {
        self.oam_palette_0.into()
    }

    pub fn get_oam_palette_1(&self) -> u8 {
        self.oam_palette_1.into()
    }

    pub fn get_window_y(&self) -> u8 {
        self.window_y
    }

    pub fn get_window_x(&self) -> u8 {
        self.window_x
    }

    pub fn set_lcd_control(&mut self, value: u8, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {
        let new_control = LcdControl::from_bits_truncate(value);

        // If we turn OFF the display
        if !new_control.contains(LcdControl::LCD_DISPLAY) && self.lcd_control.contains(LcdControl::LCD_DISPLAY) {
            log::debug!("Turning off LCD");
            self.current_y = 0;
            self.window_counter = 0;
            self.lcd_status.set_mode_flag(Mode::HBlank);
            // Turn PPU off by removing all scheduled events. TODO: Find cleaner way to do this.
            scheduler.remove_event_type(EventType::HBLANK);
            scheduler.remove_event_type(EventType::VblankWait);
            scheduler.remove_event_type(EventType::VBLANK);
            scheduler.remove_event_type(EventType::LcdTransfer);
            scheduler.remove_event_type(EventType::OamSearch);
        }
        // If we turn ON the display
        if new_control.contains(LcdControl::LCD_DISPLAY) && !self.lcd_control.contains(LcdControl::LCD_DISPLAY) {
            log::debug!("Turning on LCD");
            // Turn PPU back on. Assume pessimistic hblank timing
            self.ly_lyc_compare(interrupts);
            scheduler.push_relative(EventType::OamSearch, 204);
        }

        self.lcd_control = new_control;
    }

    pub fn set_lcd_status(&mut self, value: u8, interrupts: &mut Interrupts) {
        // Mask the 3 lower bits, which are read only and must therefore be preserved.
        let read_only_bits = self.lcd_status.bits() & 0x7;
        // For Stat IRQ blocking, note: currently not actually working (stat irq blocking that is)
        let none = self.count_currently_true_stat_interrupts() == 0;
        // Mask bit 3..=6 in case a game tries to write to the three lower bits as well.
        self.lcd_status = LcdStatus::from_bits_truncate(0x80 | (value & 0x78) | read_only_bits);

        // If we're in a mode where the interrupt should occur we need to fire those interrupts.
        // So long as there were no previous interrupts triggered.
        if none {
            self.check_all_interrupts(interrupts);
        }
    }

    pub fn set_scy(&mut self, value: u8) {
        self.scroll_y = value
    }

    pub fn set_scx(&mut self, value: u8) {
        self.scroll_x = value
    }

    pub fn set_ly(&mut self, value: u8) {
        //log::debug!("Attempted write to LY (0xFF44) when this register is read only!");
    }

    pub fn set_lyc(&mut self, value: u8, interrupts: &mut Interrupts) {
        self.compare_line = value;
        // Only update the LYC=LY STAT if the PPU is on
        // TODO: Figure out why this causes a regression in Prehistorik Man, even though it helps pass part of stat_lyc_onoff.gb
        // if self.lcd_control.contains(LcdControl::LCD_DISPLAY) {
        //     self.ly_lyc_compare(interrupts);
        //     log::warn!("Read: {:08b}", self.lcd_status.bits());
        // }
    }

    pub fn set_bg_palette(&mut self, value: u8) {
        self.bg_window_palette = Palette::from(value)
    }

    pub fn set_oam_palette_0(&mut self, value: u8) {
        self.oam_palette_0 = Palette::from(value)
    }

    pub fn set_oam_palette_1(&mut self, value: u8) {
        self.oam_palette_1 = Palette::from(value)
    }

    pub fn set_window_y(&mut self, value: u8) {
        self.window_y = value
    }

    pub fn set_window_x(&mut self, value: u8) {
        self.window_x = value
    }

    /// Checks all possible LCD STAT interrupts, and fires them
    /// if available.
    fn check_all_interrupts(&mut self, interrupts: &mut Interrupts) {
        self.ly_lyc_compare(interrupts);

        if self.lcd_status.mode_flag() == VBlank && self.lcd_status.contains(LcdStatus::MODE_1_V_INTERRUPT) {
            interrupts.insert_interrupt(InterruptFlags::LCD);
        } else if self.lcd_status.mode_flag() == OamSearch && self.lcd_status.contains(LcdStatus::MODE_2_OAM_INTERRUPT) {
            interrupts.insert_interrupt(InterruptFlags::LCD);
        } else if self.lcd_status.mode_flag() == HBlank && self.lcd_status.contains(LcdStatus::MODE_0_H_INTERRUPT) {
            interrupts.insert_interrupt(InterruptFlags::LCD);
        }
    }

    /// Returns the amount of interrupts (mode 0,1,2, ly=lc) set for LCD Stat
    pub(crate) fn count_currently_true_stat_interrupts(&self) -> u8 {
        let mut count = 0;

        if self.lcd_status.contains(LcdStatus::MODE_1_V_INTERRUPT) && self.lcd_status.mode_flag() == VBlank {
            count += 1;
        }
        if self.lcd_status.contains(LcdStatus::MODE_2_OAM_INTERRUPT) && self.lcd_status.mode_flag() == OamSearch {
            count += 1;
        }
        if self.lcd_status.contains(LcdStatus::MODE_0_H_INTERRUPT) && self.lcd_status.mode_flag() == HBlank {
            count += 1;
        }
        if self.lcd_status.contains(LcdStatus::COINCIDENCE_INTERRUPT) && self.current_y == self.compare_line {
            count += 1;
        }

        count
    }
}

/// Get the internal PPU address for a tile from a normal u16 address.
/// Returns in the format `(tile_addresss, byte_address)`
fn get_tile_address(address: u16) -> (usize, usize) {
    let relative_address = address as usize - TILE_BLOCK_0_START as usize;
    (relative_address / 16, relative_address % 16)
}
