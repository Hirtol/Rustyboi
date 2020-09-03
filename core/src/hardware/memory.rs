use crate::hardware::cartridge::Cartridge;
use crate::hardware::ppu::tiledata::{
    TILEMAP_9800_START, TILEMAP_9C00_END, TILE_BLOCK_0_START, TILE_BLOCK_2_END,
};
use crate::hardware::ppu::PPU;
use crate::io::bootrom::BootRom;
use crate::io::interrupts::InterruptFlags;
use bitflags::_core::cell::RefCell;
use bitflags::_core::fmt::{Debug, Formatter};
use log::*;
use std::fmt;
use std::rc::Rc;

pub const MEMORY_SIZE: usize = 0x10000;
/// 16 KB ROM bank, usually 00. From Cartridge, read-only
pub const ROM_BANK_00_START: u16 = 0x0000;
pub const ROM_BANK_00_END: u16 = 0x03FFF;
/// 16 KB Rom Bank 01~NN. From cartridge, switchable bank via Memory Bank. Read-only.
pub const ROM_BANK_NN_START: u16 = 0x4000;
pub const ROM_BANK_NN_END: u16 = 0x7FFF;
/// This area contains information about the program,
/// its entry point, checksums, information about the used MBC chip, the ROM and RAM sizes, etc.
pub const CARTRIDGE_HEADER_START: u16 = 0x0100;
pub const CARTRIDGE_HEADER_END: u16 = 0x014F;
/// 8 KB of VRAM, only bank 0 in Non-CGB mode. Switchable bank 0/1 in CGB mode.
pub const VRAM_START: u16 = 0x8000;
pub const VRAM_END: u16 = 0x9FFF;
/// 8 KB of External Ram, In cartridge, switchable bank if any(?). Could hold save data.
pub const EXTERNAL_RAM_START: u16 = 0xA000;
pub const EXTERNAL_RAM_END: u16 = 0xBFFF;
/// 4 KB Work RAM bank 0
pub const WRAM_BANK_00_START: u16 = 0xC000;
pub const WRAM_BANK_00_END: u16 = 0xCFFF;
/// 4 KB Work RAM bank 1~N. Only bank 1 in Non-CGB mode Switchable bank 1~7 in CGB mode.
pub const WRAM_BANK_NN_START: u16 = 0xD000;
pub const WRAM_BANK_NN_END: u16 = 0xDFFF;
/// Mirror of C000~DDFF (ECHO RAM). Typically not used
pub const ECHO_RAM_START: u16 = 0xE000;
pub const ECHO_RAM_END: u16 = 0xFDFF;
/// The amount the ECHO_RAM_ADDRESS needs to have subtracted to get to the corresponding WRAM.
pub const ECHO_RAM_OFFSET: u16 = 0x2000;
/// Sprite attribute table (OAM)
pub const OAM_ATTRIBUTE_START: u16 = 0xFE00;
pub const OAM_ATTRIBUTE_END: u16 = 0xFE9F;
/// Not usable
pub const NOT_USABLE_START: u16 = 0xFEA0;
pub const NOT_USABLE_END: u16 = 0xFEFF;
/// I/O Registers
pub const IO_START: u16 = 0xFF00;
pub const IO_END: u16 = 0xFF7F;
/// The flag used to signal that an interrupt is pending.
pub const INTERRUPTS_FLAG: u16 = 0xFF0F;
/// High Ram (HRAM)
pub const HRAM_START: u16 = 0xFF80;
pub const HRAM_END: u16 = 0xFFFE;
/// Interrupts Enable Register (IE)
pub const INTERRUPTS_ENABLE: u16 = 0xFFFF;
/// The value to return for an invalid read
pub const INVALID_READ: u8 = 0xFF;

pub mod vram;

/// Simple memory interface for reading and writing bytes, as well as determining the
/// state of the BootRom.
pub trait MemoryMapper: Debug {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);
    fn boot_rom_finished(&self) -> bool;
}

pub struct Memory {
    memory: Vec<u8>,
    boot_rom: BootRom,
    cartridge: Cartridge,
    pub ppu: PPU,
}

impl Memory {
    pub fn new(boot_rom: Option<[u8; 0x100]>, cartridge: &[u8], ppu: PPU) -> Self {
        Memory {
            memory: vec![0xFFu8; MEMORY_SIZE],
            boot_rom: BootRom::new(boot_rom),
            cartridge: Cartridge::new(cartridge),
            ppu,
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x00FF if !self.boot_rom.is_finished => self.boot_rom.read_byte(address),
            ROM_BANK_00_START..=ROM_BANK_00_END => self.cartridge.read_0000_3fff(address),
            ROM_BANK_NN_START..=ROM_BANK_NN_END => self.cartridge.read_4000_7fff(address),
            //VRAM, TODO: Address performance impact.
            TILE_BLOCK_0_START..=TILE_BLOCK_2_END => self.ppu.get_tile_byte(address),
            TILEMAP_9800_START..=TILEMAP_9C00_END => self.ppu.get_tilemap_byte(address),
            EXTERNAL_RAM_START..=EXTERNAL_RAM_END => self.memory[address as usize],
            WRAM_BANK_00_START..=WRAM_BANK_00_END => self.memory[address as usize],
            WRAM_BANK_NN_START..=WRAM_BANK_NN_END => self.memory[address as usize],
            ECHO_RAM_START..=ECHO_RAM_END => self.memory[(address - ECHO_RAM_OFFSET) as usize],
            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END => self.ppu.get_oam_byte(address),
            NOT_USABLE_START..=NOT_USABLE_END => self.non_usable_call(address),
            IO_START..=IO_END => self.read_io_byte(address),
            HRAM_START..=HRAM_END => self.memory[address as usize],
            INTERRUPTS_ENABLE => self.memory[address as usize],
            _ => self.memory[address as usize],
        }
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        let usize_address = address as usize;

        // Temporary for BLARG's tests without visual aid, this writes to the Serial port
        if address == 0xFF02 && value == 0x81 {
            println!("Output: {}", self.read_byte(0xFF01) as char);
        }

        match address {
            ROM_BANK_00_START..=ROM_BANK_NN_END => self.cartridge.write(address),
            //VRAM
            TILE_BLOCK_0_START..=TILE_BLOCK_2_END => self.ppu.set_tile_byte(address, value),
            TILEMAP_9800_START..=TILEMAP_9C00_END => self.ppu.set_tilemap_byte(address, value),

            OAM_ATTRIBUTE_START..=OAM_ATTRIBUTE_END => self.ppu.set_oam_byte(address, value),

            IO_START..=IO_END => self.write_io_byte(address, value),
            _ => self.memory[usize_address] = value,
        }
    }

    /// Specific method for all calls to the IO registers.
    /// `address` will be cast to `u8` since all registers start with `0xFF`
    fn read_io_byte(&self, address: u16) -> u8 {
        use crate::hardware::ppu::*;
        match address {
            LCD_CONTROL_REGISTER => self.ppu.get_lcd_control(),
            LCD_STATUS_REGISTER => self.ppu.get_lcd_status(),
            SCY_REGISTER => self.ppu.get_scy(),
            SCX_REGISTER => self.ppu.get_scx(),
            LY_REGISTER => self.ppu.get_ly(),
            LYC_REGISTER => self.ppu.get_lyc(),
            DMA_TRANSFER => panic!("OH NO, DMA!"), //TODO: Implement
            BG_PALETTE => self.ppu.get_bg_palette(),
            OB_PALETTE_0 => self.ppu.get_oam_palette_0(),
            OB_PALETTE_1 => self.ppu.get_oam_palette_1(),
            WY_REGISTER => self.ppu.get_window_y(),
            WX_REGISTER => self.ppu.get_window_x(),

            _ => self.memory[address as usize],
        }
    }

    fn write_io_byte(&mut self, address: u16, value: u8) {
        use crate::hardware::ppu::*;
        match address {
            LCD_CONTROL_REGISTER => self.ppu.set_lcd_control(value),
            LCD_STATUS_REGISTER => self.ppu.set_lcd_status(value),
            SCY_REGISTER => self.ppu.set_scy(value),
            SCX_REGISTER => self.ppu.set_scx(value),
            LY_REGISTER => self.ppu.set_ly(value),
            LYC_REGISTER => self.ppu.set_lyc(value),
            DMA_TRANSFER => self.dma_transfer(value),
            BG_PALETTE => self.ppu.set_bg_palette(value),
            OB_PALETTE_0 => self.ppu.set_oam_palette_0(value),
            OB_PALETTE_1 => self.ppu.set_oam_palette_1(value),
            WY_REGISTER => self.ppu.set_window_y(value),
            WX_REGISTER => self.ppu.set_window_x(value),
            0xFF50 if !self.boot_rom.is_finished => {
                self.boot_rom.is_finished = true;
                debug!("Finished executing BootRom!");
            }
            _ => self.memory[address as usize] = value,
        }
    }

    fn dma_transfer(&mut self, value: u8) {
        let address = (value as usize) << 8;
        log::debug!("OAM Transfer starting from: 0x{:04X}", address);
        self.ppu.oam_dma_transfer(&self.gather_shadow_oam(address));
    }

    fn gather_shadow_oam(&self, start_address: usize) -> Vec<u8> {
        (0..0xA0)
            .map(|i| self.read_byte((start_address + i) as u16))
            .collect()
    }

    /// Simply returns 0 while also printing a warning to the logger.
    fn non_usable_call(&self, address: u16) -> u8 {
        warn!("ROM Accessed non usable memory: {:4X}", address);
        0
    }
}

impl MemoryMapper for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.write_byte(address, value)
    }

    fn boot_rom_finished(&self) -> bool {
        self.boot_rom.is_finished
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memory: {:?}\nCartridge: {:?}",
            self.memory, self.cartridge
        )
    }
}
