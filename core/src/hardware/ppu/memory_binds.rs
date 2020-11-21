//! This module is purely used for providing access to PPU memory resources
//! to the MMU.
use crate::hardware::mmu::{INVALID_READ, OAM_ATTRIBUTE_END, OAM_ATTRIBUTE_START};
use crate::hardware::ppu::cgb_vram::CgbTileAttribute;
use crate::hardware::ppu::PPU;
use crate::print_array_raw;
use crate::scheduler::EventType::{DMATransferComplete, HBLANK, VBLANK};

use super::*;

impl PPU {
    #[inline]
    pub fn read_vram(&self, address: u16) -> u8 {
        match address {
            TILE_BLOCK_0_START..=TILE_BLOCK_2_END if self.can_access_vram() => self.get_tile_byte(address),
            TILEMAP_9800_START..=TILEMAP_9C00_END if self.can_access_vram() => self.get_tilemap_byte(address),
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END if self.can_access_oam() => self.get_oam_byte(address),
            // *** I/O Registers ***
            LCD_CONTROL_REGISTER => self.lcd_control.bits(),
            LCD_STATUS_REGISTER => 0x80 | self.lcd_status.bits(), // Bit 7 of LCD stat is always 1
            SCY_REGISTER => self.scroll_y,
            SCX_REGISTER => self.scroll_x,
            LY_REGISTER => self.current_y,
            LYC_REGISTER => self.lyc_compare,
            BG_PALETTE => self.bg_window_palette.into(),
            OB_PALETTE_0 => self.oam_palette_0.into(),
            OB_PALETTE_1 => self.oam_palette_1.into(),
            WY_REGISTER => self.window_y,
            WX_REGISTER => self.window_x,
            CGB_VRAM_BANK_REGISTER => 0xFE | self.tile_bank_currently_used,
            CGB_BACKGROUND_COLOR_INDEX => self.cgb_bg_palette_ind.get_value(),
            CGB_BACKGROUND_PALETTE_DATA if self.can_access_vram() => self.get_cgb_bg_palette_data(),
            CGB_SPRITE_COLOR_INDEX => self.cgb_sprite_palette_ind.get_value(),
            CGB_OBJECT_PALETTE_DATA if self.can_access_vram() => self.get_cgb_obj_palette_data(),
            CGB_OBJECT_PRIORITY_MODE => self.get_object_priority(),
            _ => INVALID_READ,
        }
    }

    #[inline]
    pub fn write_vram(&mut self, address: u16, value: u8, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {
        // if address != LY_REGISTER && address != LYC_REGISTER {
        //      log::warn!("Writing {:4X}, latest access: {}", address, scheduler.current_time - self.latest_access);
        //      self.latest_access = scheduler.current_time;
        // }
        match address {
            TILE_BLOCK_0_START..=TILE_BLOCK_2_END if self.can_access_vram() => self.set_tile_byte(address, value),
            TILEMAP_9800_START..=TILEMAP_9C00_END if self.can_access_vram() => self.set_tilemap_byte(address, value),
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END if self.can_access_oam() => self.set_oam_byte(address, value),
            // *** I/O Registers ***
            LCD_CONTROL_REGISTER => self.set_lcd_control(value, scheduler, interrupts),
            LCD_STATUS_REGISTER => self.set_lcd_status(value, interrupts),
            SCY_REGISTER => self.scroll_y = value,
            SCX_REGISTER => self.scroll_x = value,
            LY_REGISTER => log::debug!("ROM tried to write to LY with value: {}", value),
            LYC_REGISTER => {
                self.lyc_compare = value;
                // Ensure the comparison flag in LCD Stat is correct, so long as the PPU is on.
                if self.lcd_control.contains(LcdControl::LCD_DISPLAY) {
                    self.ly_lyc_compare(interrupts);
                }
            }
            BG_PALETTE => self.set_bg_palette(value),
            OB_PALETTE_0 => self.set_oam_palette_0(value),
            OB_PALETTE_1 => self.set_oam_palette_1(value),
            WY_REGISTER => self.window_y = value,
            WX_REGISTER => self.window_x = value,
            CGB_VRAM_BANK_REGISTER => self.tile_bank_currently_used = value & 0x1,
            CGB_BACKGROUND_COLOR_INDEX => self.cgb_bg_palette_ind.set_value(value),
            CGB_BACKGROUND_PALETTE_DATA if self.can_access_vram() => self.set_colour_bg_palette_data(value),
            CGB_SPRITE_COLOR_INDEX => self.cgb_sprite_palette_ind.set_value(value),
            CGB_OBJECT_PALETTE_DATA if self.can_access_vram() => self.set_colour_obj_palette_data(value),
            CGB_OBJECT_PRIORITY_MODE => self.set_object_priority(value),
            // Ignore writes if they're not valid
            _ => {}
        }
    }

    /// Can always access vram if PPU is disabled (then `Mode` == `Hblank`, so allowed).
    /// However, during `LcdTransfer` it's not allowed, nor is it allowed
    /// the cycle before changing to `LcdTransfer` (while still in OamTransfer).
    /// TODO: Add cycle check
    #[inline]
    fn can_access_vram(&self) -> bool {
        self.lcd_status.mode_flag() != LcdTransfer
    }

    /// Check if the OAM is currently accessible, only possible during `Hblank` and `Vblank`,
    /// or when the PPU is off.
    ///
    /// Will also block on the first cycle of every scanline. TODO: Add cycle check.
    #[inline]
    fn can_access_oam(&self) -> bool {
        let mode = self.lcd_status.mode_flag();
        mode != OamSearch && mode != LcdTransfer && !self.oam_transfer_ongoing
    }

    pub fn get_current_mode(&self) -> Mode {
        self.lcd_status.mode_flag()
    }

    fn get_tile_byte(&self, address: u16) -> u8 {
        let (tile_address, byte_address) = get_tile_address(address);
        let offset = 384 * self.tile_bank_currently_used as usize;

        self.tiles[offset + tile_address].data[byte_address]
    }

    fn set_tile_byte(&mut self, address: u16, value: u8) {
        let (tile_address, byte_address) = get_tile_address(address);
        let offset = 384 * self.tile_bank_currently_used as usize;

        self.tiles[offset + tile_address].update_pixel_data(byte_address, value);
    }

    fn get_tilemap_byte(&self, address: u16) -> u8 {
        match address {
            TILEMAP_9800_START..=TILEMAP_9800_END => {
                if self.tile_bank_currently_used == 0 {
                    self.tile_map_9800.data[(address - TILEMAP_9800_START) as usize]
                } else {
                    self.cgb_9800_tile_map.attributes[(address - TILEMAP_9800_START) as usize].bits()
                }
            }
            // 9C00, assuming no malicious calls
            _ => {
                if self.tile_bank_currently_used == 0 {
                    self.tile_map_9c00.data[(address - TILEMAP_9C00_START) as usize]
                } else {
                    self.cgb_9c00_tile_map.attributes[(address - TILEMAP_9C00_START) as usize].bits()
                }
            }
        }
    }

    fn set_tilemap_byte(&mut self, address: u16, value: u8) {
        match address {
            TILEMAP_9800_START..=TILEMAP_9800_END => {
                if self.tile_bank_currently_used == 0 {
                    self.tile_map_9800.data[(address - TILEMAP_9800_START) as usize] = value
                } else {
                    self.cgb_9800_tile_map.attributes[(address - TILEMAP_9800_START) as usize] =
                        CgbTileAttribute::from_bits_truncate(value)
                }
            }
            // 9C00, assuming no malicious calls
            _ => {
                if self.tile_bank_currently_used == 0 {
                    self.tile_map_9c00.data[(address - TILEMAP_9C00_START) as usize] = value
                } else {
                    self.cgb_9c00_tile_map.attributes[(address - TILEMAP_9C00_START) as usize] =
                        CgbTileAttribute::from_bits_truncate(value)
                }
            }
        }
    }

    fn get_oam_byte(&self, address: u16) -> u8 {
        let relative_address = (address - OAM_ATTRIBUTE_START) / 4;

        self.oam[relative_address as usize].get_byte((address % 4) as u8)
    }

    fn set_oam_byte(&mut self, address: u16, value: u8) {
        let relative_address = (address - OAM_ATTRIBUTE_START) / 4;

        self.oam[relative_address as usize].set_byte((address % 4) as u8, value);
    }

    fn get_object_priority(&self) -> u8 {
        if self.cgb_object_priority {
            0x1
        } else {
            0x0
        }
    }

    fn set_object_priority(&mut self, value: u8) {
        self.cgb_object_priority = (value & 0x1) == 1
    }

    fn get_cgb_bg_palette_data(&self) -> u8 {
        let addr = self.cgb_bg_palette_ind.selected_address;

        if addr % 2 == 0 {
            self.cgb_bg_palette[addr / 8].colours[(addr % 8) / 2].get_high_byte()
        } else {
            self.cgb_bg_palette[addr / 8].colours[(addr % 8) / 2].get_low_byte()
        }
    }

    fn set_colour_bg_palette_data(&mut self, value: u8) {
        let addr = self.cgb_bg_palette_ind.selected_address;

        if addr % 2 == 0 {
            self.cgb_bg_palette[addr / 8].colours[(addr % 8) / 2].set_low_byte(value);
        } else {
            self.cgb_bg_palette[addr / 8].colours[(addr % 8) / 2].set_high_byte(value);
        }

        if self.cgb_bg_palette_ind.auto_increment {
            self.cgb_bg_palette_ind.selected_address = addr.wrapping_add(1) % 64;
        }
    }

    fn get_cgb_obj_palette_data(&self) -> u8 {
        let addr = self.cgb_sprite_palette_ind.selected_address;

        if addr % 2 == 0 {
            self.cgb_sprite_palette[addr / 8].colours[(addr % 8) / 2].get_high_byte()
        } else {
            self.cgb_sprite_palette[addr / 8].colours[(addr % 8) / 2].get_low_byte()
        }
    }

    fn set_colour_obj_palette_data(&mut self, value: u8) {
        let addr = self.cgb_sprite_palette_ind.selected_address;

        if addr % 2 == 0 {
            self.cgb_sprite_palette[addr / 8].colours[(addr % 8) / 2].set_low_byte(value);
        } else {
            self.cgb_sprite_palette[addr / 8].colours[(addr % 8) / 2].set_high_byte(value);
        }

        if self.cgb_sprite_palette_ind.auto_increment {
            self.cgb_sprite_palette_ind.selected_address = addr.wrapping_add(1) % 64;
        }
    }

    fn set_lcd_control(&mut self, value: u8, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {
        let was_lcd_on = self.lcd_control.contains(LcdControl::LCD_DISPLAY);
        self.lcd_control = LcdControl::from_bits_truncate(value);
        // If we turn OFF the display
        if !self.lcd_control.contains(LcdControl::LCD_DISPLAY) && was_lcd_on {
            self.turn_off_lcd(scheduler);
        } else if self.lcd_control.contains(LcdControl::LCD_DISPLAY) && !was_lcd_on {
            self.turn_on_lcd(scheduler, interrupts);
        }
    }

    pub fn turn_off_lcd(&mut self, scheduler: &mut Scheduler) {
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

    pub fn turn_on_lcd(&mut self, scheduler: &mut Scheduler, interrupts: &mut Interrupts) {
        log::debug!("Turning on LCD");
        self.ly_lyc_compare(interrupts);
        // Turn PPU back on. The first line is very funky, as we skip OamSearch entirely
        // and skip to LcdTransfer instead after 76 cycles (unconfirmed exact amount).
        // Seems we should end the line 4 cycles early as well?
        scheduler.push_relative(EventType::LcdTransfer, 76);
    }

    fn set_lcd_status(&mut self, value: u8, interrupts: &mut Interrupts) {
        // Mask the 3 lower bits, which are read only and must therefore be preserved.
        let read_only_bits = self.lcd_status.bits() & 0x7;
        // Mask bit 3..=6 in case a game tries to write to the three lower bits as well.
        self.lcd_status = LcdStatus::from_bits_truncate(0x80 | (value & 0x78) | read_only_bits);

        self.request_stat_interrupt(interrupts);
    }

    pub fn update_display_colours(
        &mut self,
        bg_palette: DisplayColour,
        sp0_palette: DisplayColour,
        sp1_palette: DisplayColour,
        emu_mode: GameBoyModel,
    ) {
        // We don't want to overwrite CGB registers if we're actually running a CGB game.
        if emu_mode.is_dmg() {
            let (cgb_bg_palette, cgb_sprite_palette) = initialise_cgb_palette(bg_palette, sp0_palette, sp1_palette);
            self.cgb_bg_palette = cgb_bg_palette;
            self.cgb_sprite_palette = cgb_sprite_palette;
            self.set_bg_palette(self.bg_window_palette.into());
            self.set_oam_palette_0(self.oam_palette_0.into());
            self.set_oam_palette_1(self.oam_palette_1.into());
        }
    }

    fn set_bg_palette(&mut self, value: u8) {
        self.bg_window_palette = Palette::new(value, DisplayColour::from(self.cgb_bg_palette[0].rgb()))
    }

    fn set_oam_palette_0(&mut self, value: u8) {
        self.oam_palette_0 = Palette::new(value, DisplayColour::from(self.cgb_sprite_palette[0].rgb()))
    }

    fn set_oam_palette_1(&mut self, value: u8) {
        self.oam_palette_1 = Palette::new(value, DisplayColour::from(self.cgb_sprite_palette[1].rgb()))
    }

    /// Checks which interrupt(s) should fire, and if there are any, check for a rising
    /// edge for the actual LCD Stat interrupt.
    pub fn request_stat_interrupt(&mut self, interrupts: &mut Interrupts) {
        if !self.lcd_control.contains(LcdControl::LCD_DISPLAY) {
            return;
        }

        let old_stat_irq = self.stat_irq_triggered;

        self.stat_irq_triggered = match self.get_current_mode() {
            HBlank => self.lcd_status.contains(LcdStatus::MODE_0_H_INTERRUPT),
            VBlank if self.emulated_model.is_dmg() => {
                self.lcd_status.contains(LcdStatus::MODE_1_V_INTERRUPT)
                    || self.lcd_status.contains(LcdStatus::MODE_2_OAM_INTERRUPT)
            }
            VBlank if self.emulated_model.is_cgb() => self.lcd_status.contains(LcdStatus::MODE_1_V_INTERRUPT),
            OamSearch => self.lcd_status.contains(LcdStatus::MODE_2_OAM_INTERRUPT),
            _ => false,
        };

        // If Ly=Lyc we want to reactivate this interrupt.
        if self.lcd_status.contains(LcdStatus::COINCIDENCE_INTERRUPT) && self.current_y == self.lyc_compare {
            self.stat_irq_triggered = true;
        }
        // Only on a rising edge do we want to trigger the LCD interrupts.
        if !old_stat_irq && self.stat_irq_triggered {
            interrupts.insert_interrupt(InterruptFlags::LCD);
        }
    }
}

/// Get the internal PPU address for a tile from a normal u16 address.
/// Returns in the format `(tile_addresss, byte_address)`
fn get_tile_address(address: u16) -> (usize, usize) {
    let relative_address = (address - TILE_BLOCK_0_START) as usize;
    (relative_address / 16, relative_address % 16)
}
