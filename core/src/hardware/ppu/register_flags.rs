use crate::hardware::ppu::Mode;

use bitflags::*;
use crate::hardware::ppu::memory_binds::{TILE_BLOCK_0_START, TILE_BLOCK_1_START};

// # PPU FLAGS #

bitflags! {
    /// FF40
    /// LCDC is a powerful tool: each bit controls a lot of behavior,
    /// and can be modified at any time during the frame.
    ///
    /// One of the important aspects of LCDC is that unlike VRAM,
    /// the PPU never locks it. It's thus possible to modify it mid-scanline!
    #[derive(Default)]
    pub struct LcdControl: u8 {
        /// `BG_WINDOW_PRIORITY` has different meanings depending on Game Boy type and Mode:
        ///# Monochrome Gameboy, SGB and CGB in Non-CGB Mode: BG Display
        /// When Bit 0 is cleared, both background and window become blank (white),
        /// and the Window Display Bit is ignored in that case.
        /// Only Sprites may still be displayed (if enabled in Bit 1).
        ///
        ///# CGB in CGB Mode: BG and Window Master Priority
        /// When Bit 0 is cleared, the background and window lose their priority -
        /// the sprites will be always displayed on top of background and window,
        /// independently of the priority flags in OAM and BG Map attributes.
        const BG_WINDOW_PRIORITY = 0b0000_0001;
        /// This bit toggles whether sprites are displayed or not.
        /// This can be toggled mid-frame, for example to avoid sprites
        /// being displayed on top of a status bar or text box.
        const SPRITE_DISPLAY_ENABLE    = 0b0000_0010;
        /// This bit controls the sprite size (1 tile or 2 stacked vertically).
        /// Be cautious when changing this mid-frame from 8x8 to 8x16:
        /// "remnants" of the sprites intended for 8x8
        /// could "leak" into the 8x16 zone and cause artifacts.
        const SPRITE_SIZE  = 0b0000_0100;
        /// LCDC.3
        /// This bit works similarly to `WINDOW_TILE_SELECT`: if the bit is reset,
        /// the BG uses tilemap `$9800`, otherwise tilemap `$9C00`.
        const BG_TILE_MAP_SELECT = 0b0000_1000;
        /// LCDC.4
        /// 0=8800-97FF, 1=8000-8FFF
        /// This bit controls which addressing mode the BG and Window use to pick tiles.
        /// Sprites aren't affected by this, and will always use $8000 addressing mode.
        const BG_WINDOW_TILE_SELECT = 0b0001_0000;
        /// This bit controls whether the window shall be displayed or not.
        /// This bit is overridden on DMG by bit 0 (`BG_WINDOW_PRIORITY`) if that bit is reset.
        const WINDOW_DISPLAY = 0b0010_0000;
        /// LCDC.6
        /// This bit controls which background map the Window uses for rendering.
        /// When it's reset (0), the `$9800` tilemap is used, otherwise it's the `$9C00` one.
        const WINDOW_MAP_SELECT = 0b0100_0000;
        /// This bit controls whether the LCD is on and the PPU is active.
        /// Setting it to 0 turns both off, which grants immediate and full access to VRAM, OAM.
        const LCD_DISPLAY = 0b1000_0000;
    }
}

bitflags! {
    /// FF41
    /// A dot is the shortest period over which the PPU can output one pixel:
    /// it is equivalent to 1 T-state on DMG or on CGB single-speed mode or 2 T-states on
    /// CGB double-speed mode.
    /// On each dot during mode 3, either the PPU outputs a pixel
    /// or the fetcher is stalling the FIFOs.
    #[derive(Default)]
    pub struct LcdStatus: u8 {
        /// Mode Flag       (Mode 0-3, see below) (Read Only)
        /// * 0: During H-Blank
        /// * 1: During V-Blank
        /// * 2: During Searching OAM
        /// * 3: During Transferring Data to LCD Driver
        const MODE_FLAG_0 = 0b0000_0001;
        /// Extension of `MODE_FLAG_0`
        const MODE_FLAG_1    = 0b0000_0010;
        /// (0:LYC<>LY, 1:LYC=LY) (Read Only)
        const COINCIDENCE_FLAG  = 0b0000_0100;
        /// Mode 0 H-Blank Interrupt
        const MODE_0_H_INTERRUPT = 0b0000_1000;
        /// Mode 1 V-Blank Interrupt
        const MODE_1_V_INTERRUPT = 0b0001_0000;
        /// Mode 2 OAM Interrupt
        const MODE_2_OAM_INTERRUPT = 0b0010_0000;
        /// LYC=LY Coincidence Interrupt (1=Enable) (Read/Write)
        const COINCIDENCE_INTERRUPT = 0b0100_0000;

        const UNUSED = 0b1000_0000;
    }
}

bitflags! {
    /// FF41
    /// A dot is the shortest period over which the PPU can output one pixel:
    /// it is equivalent to 1 T-state on DMG or on CGB single-speed mode or 2 T-states on
    /// CGB double-speed mode.
    /// On each dot during mode 3, either the PPU outputs a pixel
    /// or the fetcher is stalling the FIFOs.
    #[derive(Default)]
    pub struct AttributeFlags: u8 {
        /// **CGB Mode Only**     (OBP0-7)
        const PALETTE_NUMBER_CGB = 0b0000_0111;
        /// **CGB Mode Only**     (0=Bank 0, 1=Bank 1)
        const TILE_VRAM_BANK = 0b0000_1000;
        /// **Non CGB Mode Only** (0=OBP0, 1=OBP1)
        const PALETTE_NUMBER = 0b0001_0000;
        /// (0=Normal, 1=Horizontally mirrored)
        const X_FLIP = 0b0010_0000;
        /// (0=Normal, 1=Vertically mirrored)
        const Y_FLIP = 0b0100_0000;
        /// (0=OBJ Above BG, 1=OBJ Behind BG color 1-3)
        /// (Used for both BG and Window. BG color 0 is always behind OBJ)
        const OBJ_TO_BG_PRIORITY = 0b1000_0000;
    }
}

impl AttributeFlags {
    pub fn get_cgb_palette_number(&self) -> usize {
        (self.bits & 0x07) as usize
    }
}

impl LcdControl {
    pub fn bg_window_tile_address(&self) -> u16 {
        if self.contains(LcdControl::BG_WINDOW_TILE_SELECT) {
            TILE_BLOCK_0_START
        } else {
            TILE_BLOCK_1_START
        }
    }
}

impl LcdStatus {
    pub fn mode_flag(&self) -> Mode {
        match self.bits & 0x3 {
            0 => Mode::Hblank,
            1 => Mode::Vblank,
            2 => Mode::OamSearch,
            3 => Mode::LcdTransfer,
            _ => unreachable!("Invalid value entered for mode flag"),
        }
    }

    pub fn set_mode_flag(&mut self, value: Mode) {
        self.bits = (self.bits & 0xFC)
            | match value {
                Mode::Hblank => 0,
                Mode::Vblank => 1,
                Mode::OamSearch => 2,
                Mode::LcdTransfer => 3,
            }
    }
}
